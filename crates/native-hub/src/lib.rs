//! Experimental integration of the upstream Moli OS hub as a laboratory-only
//! Rust-native state authority. Zero network listeners, no production devices.
//! This is not a Home Assistant replacement or a release-ready service.
pub mod haos_guest;
pub mod oci_quadlet;
use moli_core::{DeviceId, PointId, Sample};
use moli_runtime::{Hub, HubOptions, Snapshot};
use std::collections::{BTreeMap, HashMap};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Thin handle over the unmodified upstream hub, with device registration
/// intentionally unavailable from the public façade until access-controlled
/// driver lifecycle and one-writer ownership are proven.
pub struct NativeHubReadOnly {
    hub: Hub,
    state_cache: Option<PathBuf>,
    checkpoint_lock: Arc<tokio::sync::Mutex<()>>,
}
impl NativeHubReadOnly {
    pub fn new(state_cache: Option<PathBuf>) -> io::Result<Self> {
        if let Some(path) = &state_cache {
            validate_existing_cache(path)?;
        }
        let hub = Hub::new(HubOptions {
            state_path: state_cache.clone(),
            ..HubOptions::default()
        })?;
        Ok(Self {
            hub,
            state_cache,
            checkpoint_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.hub.snapshot()
    }
    pub fn state(&self, point: &PointId) -> Option<Sample> {
        self.hub.state(point)
    }
    /// Cache flushed ONLY after a real registered driver writes. This is not
    /// a transactional guarantee for abrupt power loss.
    pub async fn persist_cache(&self) -> io::Result<bool> {
        checkpoint(
            &self.hub,
            self.state_cache.as_deref(),
            &self.checkpoint_lock,
        )
        .await
    }
    /// Repeat the existing upstream Moli OS host pattern: periodic dirty
    /// cache flush, then a final flush on graceful cancellation. This is a
    /// **lab-only** task, not a durable transactional event log. The normal
    /// Moli executable itself schedules housekeeping separately.
    pub fn start_cache_housekeeping(
        &self,
        interval: Duration,
        cancel: CancellationToken,
    ) -> io::Result<JoinHandle<io::Result<()>>> {
        if interval.is_zero() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "zero interval"));
        }
        let hub = self.hub.clone();
        let path = self.state_cache.clone();
        let guard = Arc::clone(&self.checkpoint_lock);
        Ok(tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            tick.tick().await;
            loop {
                tokio::select! {
                    () = cancel.cancelled() => break,
                    _ = tick.tick() => {
                        checkpoint(&hub, path.as_deref(), &guard).await?;
                    }
                }
            }
            checkpoint(&hub, path.as_deref(), &guard).await?;
            Ok(())
        }))
    }
    #[cfg(test)]
    fn synthetic_test_hub(&self) -> Hub {
        self.hub.clone()
    }
}
/// Strict, read-only preflight. Moli's upstream Hub::new deliberately
/// discards corrupt state caches and starts empty (it views them as optional
/// caches). For migration/shadow use, silent loss is unsafe: fail closed
/// without changing or overwriting the original bytes.
fn validate_existing_cache(path: &Path) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if !meta.file_type().is_file() || meta.len() > 64 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cache must be a regular file <=64 MiB",
        ));
    }
    let data = std::fs::read(path)?;
    // Expected upstream States type. Not merely "valid JSON": reject bad
    // typed samples, IDs or timestamps instead of quietly starting empty.
    let _: HashMap<DeviceId, BTreeMap<Arc<str>, Sample>> =
        serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(())
}

/// Serializes cache writes (Moli persists atomically via rename), then asks
/// the kernel to sync the file and its parent directory. This strengthens
/// crash persistence on supporting local filesystems but is NOT proof of
/// survival after physical power loss or a lying disk controller.
async fn checkpoint(
    hub: &Hub,
    path: Option<&Path>,
    guard: &tokio::sync::Mutex<()>,
) -> io::Result<bool> {
    let _serial = guard.lock().await;
    let changed = hub.persist_state().await?;
    if !changed {
        return Ok(false);
    }
    if let Some(file) = path {
        let file = file.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::fs::File::open(&file)?.sync_all()?;
            if let Some(parent) = file.parent() {
                std::fs::File::open(parent)?.sync_all()?;
            }
            Ok::<(), io::Error>(())
        })
        .await
        .map_err(|e| io::Error::other(format!("sync task failed: {e}")))??;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moli_core::{Access, Device, InstanceId, Kind, PointSpec, Semantic, Value};
    use moli_runtime::{BoxFuture, Driver, DriverCtx, spawn_driver};
    use std::sync::Arc;
    use tokio::sync::oneshot;
    use tokio_util::sync::CancellationToken;

    struct SyntheticDriver {
        ready: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
        emit_state: bool,
    }
    impl Driver for SyntheticDriver {
        fn kind(&self) -> &'static str {
            "hars-synthetic"
        }
        fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, anyhow::Result<()>> {
            Box::pin(async move {
                for i in 0..16 {
                    let id = ctx.device_id(&format!("sensor_{i}"));
                    ctx.upsert_device(Device {
                        id: id.clone(),
                        instance: ctx.instance().clone(),
                        native_name: format!("Mock {i}").into(),
                        manufacturer: None,
                        model: None,
                        description: None,
                        native_room: None,
                        members: vec![],
                        points: vec![PointSpec {
                            key: "state".into(),
                            label: "Mock value".into(),
                            kind: Kind::Numeric {
                                min: None,
                                max: None,
                                step: None,
                            },
                            access: Access {
                                read: true,
                                write: false,
                            },
                            unit: None,
                            semantic: Semantic::Other,
                        }],
                    });
                    if self.emit_state {
                        ctx.set_state(&id, "state", Value::Int(i as i64));
                    }
                }
                ctx.ready();
                if let Some(tx) = self.ready.lock().await.take() {
                    let _ = tx.send(());
                }
                std::future::pending::<()>().await;
                Ok(())
            })
        }
    }
    async fn start_mock(hub: &Hub, emit: bool) -> (CancellationToken, tokio::task::JoinHandle<()>) {
        let (tx, rx) = oneshot::channel();
        let cancel = CancellationToken::new();
        let driver = Arc::new(SyntheticDriver {
            ready: tokio::sync::Mutex::new(Some(tx)),
            emit_state: emit,
        });
        let task = spawn_driver(hub, InstanceId::from("sandbox"), driver, cancel.clone());
        tokio::time::timeout(std::time::Duration::from_secs(4), rx)
            .await
            .unwrap()
            .unwrap();
        (cancel, task)
    }
    #[tokio::test]
    async fn isolated_upstream_hub_restores_16_synthetic_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("native-hub-cache.json");
        let original = NativeHubReadOnly::new(Some(path.clone())).unwrap();
        assert_eq!(original.snapshot().devices.len(), 0);
        let (cancel, task) = start_mock(&original.synthetic_test_hub(), true).await;
        assert_eq!(original.snapshot().devices.len(), 16);
        assert!(original.persist_cache().await.unwrap());
        cancel.cancel();
        task.await.unwrap();
        drop(original);
        let reloaded = NativeHubReadOnly::new(Some(path)).unwrap();
        let (cancel, task) = start_mock(&reloaded.synthetic_test_hub(), false).await;
        for i in 0..16 {
            let id = moli_core::DeviceId::new(&InstanceId::from("sandbox"), &format!("sensor_{i}"));
            let point = PointId::new(&id, "state");
            assert_eq!(reloaded.state(&point).unwrap().value, Value::Int(i as i64));
        }
        cancel.cancel();
        task.await.unwrap();
    }
    #[tokio::test]
    async fn lab_housekeeping_flushes_periodically_and_on_cancel() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("periodic.json");
        let hub = NativeHubReadOnly::new(Some(path.clone())).unwrap();
        assert!(
            hub.start_cache_housekeeping(Duration::ZERO, CancellationToken::new())
                .is_err()
        );
        let token = CancellationToken::new();
        let housekeeper = hub
            .start_cache_housekeeping(Duration::from_millis(25), token.clone())
            .unwrap();
        let (driver_cancel, driver_task) = start_mock(&hub.synthetic_test_hub(), true).await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if path.is_file() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        token.cancel();
        housekeeper.await.unwrap().unwrap();
        driver_cancel.cancel();
        driver_task.await.unwrap();
        drop(hub);
        let reload = NativeHubReadOnly::new(Some(path)).unwrap();
        let (cancel, task) = start_mock(&reload.synthetic_test_hub(), false).await;
        let id = moli_core::DeviceId::new(&InstanceId::from("sandbox"), "sensor_0");
        assert_eq!(
            reload.state(&PointId::new(&id, "state")).unwrap().value,
            Value::Int(0)
        );
        cancel.cancel();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn lagged_event_subscriber_must_resync_from_snapshot() {
        use tokio::sync::broadcast::error::TryRecvError;
        let hub = Hub::new(HubOptions {
            event_capacity: 16,
            ..HubOptions::default()
        })
        .unwrap();
        let mut events = hub.subscribe();
        let (cancel, task) = start_mock(&hub, true).await;
        // The fake driver publishes 16 devices and 16 state updates.
        // A 16-slot ring cannot retain all events for a slow subscriber.
        assert!(matches!(events.try_recv(), Err(TryRecvError::Lagged(_))));
        let snap = hub.snapshot();
        assert_eq!(snap.devices.len(), 16);
        for i in 0..16 {
            let id = moli_core::DeviceId::new(&InstanceId::from("sandbox"), &format!("sensor_{i}"));
            assert_eq!(
                hub.state(&PointId::new(&id, "state")).unwrap().value,
                Value::Int(i as i64)
            );
        }
        cancel.cancel();
        task.await.unwrap();
    }
    #[tokio::test]
    async fn without_explicit_cache_persist_reboot_loses_unflushed_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("not-persisted.json");
        let hub = NativeHubReadOnly::new(Some(path.clone())).unwrap();
        let (cancel, task) = start_mock(&hub.synthetic_test_hub(), true).await;
        cancel.cancel();
        task.await.unwrap();
        drop(hub);
        assert!(
            !path.exists(),
            "No implicit state cache flush should be assumed"
        );
        let restarted = NativeHubReadOnly::new(Some(path)).unwrap();
        let (cancel, task) = start_mock(&restarted.synthetic_test_hub(), false).await;
        let id = moli_core::DeviceId::new(&InstanceId::from("sandbox"), "sensor_0");
        assert!(restarted.state(&PointId::new(&id, "state")).is_none());
        cancel.cancel();
        task.await.unwrap();
    }
    #[test]
    fn facade_rejects_corrupt_cache_without_overwriting_bytes() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("corrupt-state.json");
        let corrupt = b"TRUNCATED_OR_INVALID_JSON";
        std::fs::write(&path, corrupt).unwrap();
        let err = NativeHubReadOnly::new(Some(path.clone()))
            .err()
            .expect("strict preflight should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        // Moli upstream's Hub::new intentionally treats corrupt cache as
        // optional and starts empty. Our wrapper must not do so.
        let upstream = Hub::new(HubOptions {
            state_path: Some(path.clone()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(upstream.snapshot().devices.len(), 0);
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }

    #[cfg(unix)]
    #[test]
    fn facade_refuses_symbolic_cache_path() {
        let scratch = tempfile::tempdir().unwrap();
        let real = scratch.path().join("real.json");
        std::fs::write(&real, b"{}").unwrap();
        let link = scratch.path().join("link.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let err = NativeHubReadOnly::new(Some(link)).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(&real).unwrap(), b"{}");
    }

    #[tokio::test]
    async fn no_device_writer_means_no_state_cache_dirty() {
        let hub = NativeHubReadOnly::new(None).unwrap();
        assert!(hub.snapshot().devices.is_empty());
        assert!(!hub.persist_cache().await.unwrap());
    }
}
