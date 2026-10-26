//! Generic OCI App -> native Podman Quadlet declarative units, lab only.
//! No privileged containers, host network/device maps, or app-specific allowlists.
//! This is NOT a complete HA Supervisor App manifest translator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuadletError {
    InvalidSlug,
    UnpinnedImage,
    InvalidResources,
}

/// Localhost-only service access requires an explicit operator-selected port.
/// slirp4netns may still allow OUTBOUND egress; not an outbound firewall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppNetwork {
    Offline,
    LoopbackTcp { host_port: u16, container_port: u16 },
}
pub struct OciApp<'a> {
    pub slug: &'a str,
    pub image: &'a str,
    pub memory_mib: u16,
    pub pids_limit: u16,
    pub run_uid: Option<u32>,
    pub network: AppNetwork,
}
pub struct QuadletUnits {
    pub container_name: String,
    pub container: String,
    pub volume_name: String,
    pub volume: String,
}
/// Bind only an EXISTING rootless Podman secret by its safe opaque name.
/// The secret material is never written to a Quadlet, argv, or app manifest.
/// The caller must verify Podman secret existence in the matching user store.
pub fn bind_native_secret(units: &mut QuadletUnits, name: &str) -> Result<(), QuadletError> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_lowercase()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        || units.container.matches("Pull=never\n").count() != 1
        || units.container.contains("Secret=")
    {
        return Err(QuadletError::InvalidResources);
    }
    units.container =
        units
            .container
            .replacen("Pull=never\n", &format!("Pull=never\nSecret={name}\n"), 1);
    Ok(())
}

impl OciApp<'_> {
    /// A Podman-managed named volume, never a host-path bind.
    /// Custom OCI storage destinations must be absolute safe container paths.
    pub fn render_at(&self, destination: &str) -> Result<QuadletUnits, QuadletError> {
        if destination.len() < 2
            || destination.len() > 120
            || !destination.starts_with('/')
            || destination.ends_with('/')
            || destination.contains("//")
            || destination.contains('\\')
            || !destination.split('/').skip(1).all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.')
            })
            || ["/proc", "/sys", "/dev", "/etc", "/run", "/root"]
                .iter()
                .any(|reserved| {
                    destination == *reserved || destination.starts_with(&format!("{reserved}/"))
                })
        {
            return Err(QuadletError::InvalidResources);
        }
        let mut q = self.render()?;
        let needle = format!(
            "Volume={}.volume:/data",
            q.volume_name.trim_end_matches(".volume")
        );
        if q.container.matches(&needle).count() != 1 {
            return Err(QuadletError::InvalidResources);
        }
        q.container = q.container.replacen(
            &needle,
            &format!(
                "Volume={}.volume:{destination}",
                q.volume_name.trim_end_matches(".volume")
            ),
            1,
        );
        Ok(q)
    }
    pub fn render(&self) -> Result<QuadletUnits, QuadletError> {
        let slug = self.slug;
        if slug.len() > 64
            || slug.is_empty()
            || !slug.as_bytes()[0].is_ascii_lowercase()
            || !slug
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err(QuadletError::InvalidSlug);
        }
        if !(32..=8192).contains(&self.memory_mib) || !(8..=4096).contains(&self.pids_limit) {
            return Err(QuadletError::InvalidResources);
        }
        let Some((repository, digest)) = self.image.split_once("@sha256:") else {
            return Err(QuadletError::UnpinnedImage);
        };
        if repository.len() < 5
            || repository.len() > 240
            || !repository.contains('/')
            || repository.contains("//")
            || repository.contains("..")
            || repository.starts_with('/')
            || repository.ends_with('/')
            || !repository
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-:/".contains(&c))
            || repository.split('/').any(str::is_empty)
            || digest.len() != 64
            || !digest
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(QuadletError::UnpinnedImage);
        }
        if self.run_uid.is_some_and(|uid| uid == 0 || uid > 65533) {
            return Err(QuadletError::InvalidResources);
        }
        let net = match self.network {
            AppNetwork::Offline => String::from("Network=none\n"),
            AppNetwork::LoopbackTcp {
                host_port,
                container_port,
            } => {
                if host_port < 1024 || container_port == 0 {
                    return Err(QuadletError::InvalidResources);
                }
                format!(
                    "Network=slirp4netns\nPublishPort=127.0.0.1:{host_port}:{container_port}/tcp\n"
                )
            }
        };
        let explicit_user = self
            .run_uid
            .map(|uid| format!("User={uid}\n"))
            .unwrap_or_default();
        let mount_flags = if self.run_uid.is_some() { ":U" } else { "" };
        let container_name = format!("rh-{slug}");
        let volume_name = format!("{container_name}-data");
        let container = format!(
            "[Unit]\nDescription=Rusty Home isolated OCI App {slug} (lab only)\n\n\
             [Container]\nImage={image}\nContainerName={container_name}\n\
             Volume={volume_name}.volume:/data{mount_flags}\n{explicit_user}{net}Pull=never\n\
             NoNewPrivileges=true\nDropCapability=all\nReadOnly=true\n\
             PidsLimit={pids}\n\n\
             [Service]\nMemoryMax={memory}M\nRestart=on-failure\nRestartSec=5s\nTimeoutStartSec=180s\n",
            image = self.image,
            pids = self.pids_limit,
            memory = self.memory_mib,
            mount_flags = mount_flags,
            explicit_user = explicit_user,
            net = net
        );
        let volume = format!(
            "[Unit]\nDescription=Rusty Home isolated OCI App {slug} persistent volume\n\n\
             [Volume]\nVolumeName={volume_name}\n"
        );
        Ok(QuadletUnits {
            container_name: format!("{container_name}.container"),
            container,
            volume_name: format!("{volume_name}.volume"),
            volume,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    fn good<'a>(slug: &'a str, image: &'a str) -> OciApp<'a> {
        OciApp {
            slug,
            image,
            memory_mib: 256,
            pids_limit: 128,
            run_uid: None,
            network: AppNetwork::Offline,
        }
    }
    #[test]
    fn optional_native_secret_reference_never_embeds_material_or_shell() {
        let image = format!("docker.io/library/redis@sha256:{DIGEST}");
        let mut u = good("redis", &image).render().unwrap();
        bind_native_secret(&mut u, "app-secret_2").unwrap();
        assert!(u.container.contains("Secret=app-secret_2\n"));
        assert!(u.container.contains("Network=none\n"));
        assert!(u.container.contains("Pull=never\n"));
        assert!(!u.container.contains("Environment="));
        assert!(bind_native_secret(&mut u, "second").is_err());
        for name in [
            "",
            "A",
            "bad/name",
            "../../data",
            "a\n[Service]",
            "a=val",
            "a;true",
            "-leading",
            "a%.env",
            "a\n",
        ] {
            let mut clean = good("redis", &image).render().unwrap();
            assert!(bind_native_secret(&mut clean, name).is_err(), "{name}");
        }
        let mut clean = good("redis", &image).render().unwrap();
        assert!(bind_native_secret(&mut clean, &"a".repeat(65)).is_err());
    }
    #[test]
    fn any_safe_slug_and_registry_can_generate_rootless_quadlet() {
        for slug in ["mealie", "zigbee2mqtt", "matter-server", "some-app-v99"] {
            let image = format!("registry.example.org/apps/{slug}@sha256:{DIGEST}");
            let units = good(slug, &image).render().unwrap();
            assert!(units.container.contains("Network=none"));
            assert!(units.container.contains("NoNewPrivileges=true"));
            assert!(units.container.contains("DropCapability=all"));
            assert!(units.container.contains("ReadOnly=true"));
            assert!(units.container.contains("Pull=never"));
            assert!(!units.container.contains("Privileged=true"));
            assert!(!units.container.contains("[Install]"));
            assert!(
                units
                    .container
                    .contains(&format!("Volume=rh-{slug}-data.volume:/data"))
            );
        }
    }
    #[test]
    fn injection_missing_digest_and_host_paths_are_rejected() {
        for slug in [
            "",
            "root",
            "A",
            "x.y",
            "../boot",
            "x\n[Install]",
            "x/y",
            "x=y",
            "x%u",
        ] {
            // "root" is syntactically okay, app is isolated by user policy.
            if slug == "root" {
                continue;
            }
            let image = format!("docker.io/library/busybox@sha256:{DIGEST}");
            assert!(good(slug, &image).render().is_err(), "{slug}");
        }
        for image in [
            "docker.io/library/busybox:latest",
            "docker.io/library/busybox@sha256:abc",
            "docker.io/library/busybox@sha256:ZZZZ",
            "docker.io/lib\n[Service]@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "/host/image@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ] {
            assert!(good("app", image).render().is_err(), "{image}")
        }
    }
    #[test]
    fn optional_numeric_nonroot_uid_owns_volume_without_host_paths() {
        let image = format!("docker.io/library/redis@sha256:{DIGEST}");
        let mut cfg = good("redis-app", &image);
        cfg.run_uid = Some(999);
        let q = cfg.render().unwrap();
        assert!(q.container.contains("User=999\n"));
        assert!(q.container.contains(".volume:/data:U"));
        cfg.run_uid = Some(0);
        assert!(cfg.render().is_err());
        cfg.run_uid = Some(99999);
        assert!(cfg.render().is_err());
    }
    #[test]
    fn loopback_port_requires_opt_in_and_never_binds_public_interface() {
        let image = format!("docker.io/library/redis@sha256:{DIGEST}");
        let mut app = good("redis-local", &image);
        assert!(app.render().unwrap().container.contains("Network=none\n"));
        app.network = AppNetwork::LoopbackTcp {
            host_port: 18879,
            container_port: 6379,
        };
        let unit = app.render().unwrap().container;
        assert!(unit.contains("PublishPort=127.0.0.1:18879:6379/tcp"));
        assert!(unit.contains("Network=slirp4netns"));
        assert!(!unit.contains("0.0.0.0:18879"));
        app.network = AppNetwork::LoopbackTcp {
            host_port: 80,
            container_port: 6379,
        };
        assert!(app.render().is_err());
        app.network = AppNetwork::LoopbackTcp {
            host_port: 18879,
            container_port: 0,
        };
        assert!(app.render().is_err());
    }
    #[test]
    fn generic_data_path_is_container_only_absolute_and_injection_safe() {
        let image = format!("docker.io/library/registry@sha256:{DIGEST}");
        let app = good("registry", &image);
        let q = app.render_at("/var/lib/registry").unwrap();
        assert!(
            q.container
                .contains("Volume=rh-registry-data.volume:/var/lib/registry")
        );
        assert!(q.container.contains("Network=none"));
        assert!(!q.container.contains(":/data"));
        for invalid in [
            "",
            "/",
            "relative",
            "/data/",
            "/data//secret",
            "/../mnt",
            "/var/lib/../../host",
            "/etc/shadow",
            "/proc",
            "/run/secrets",
            "/var/lib/registry:Z",
            "/var/lib/registry\\n[Install]",
            "/root/config",
        ] {
            assert!(app.render_at(invalid).is_err(), "{invalid}");
        }
        assert!(app.render_at("/data").is_ok());
    }
    #[test]
    fn unreasonable_limits_are_rejected() {
        let image = format!("docker.io/library/busybox@sha256:{DIGEST}");
        let mut cfg = good("generic-app", &image);
        cfg.memory_mib = 0;
        assert!(cfg.render().is_err());
        cfg.memory_mib = 256;
        cfg.pids_limit = 1;
        assert!(cfg.render().is_err());
    }
}
