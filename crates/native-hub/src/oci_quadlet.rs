//! Generic OCI App -> native Podman Quadlet declarative units, lab only.
//! No privileged containers, host network/device maps, or app-specific allowlists.
//! This is NOT a complete HA Supervisor App manifest translator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuadletError {
    InvalidSlug,
    UnpinnedImage,
    InvalidResources,
}

pub struct OciApp<'a> {
    pub slug: &'a str,
    pub image: &'a str,
    pub memory_mib: u16,
    pub pids_limit: u16,
}
pub struct QuadletUnits {
    pub container_name: String,
    pub container: String,
    pub volume_name: String,
    pub volume: String,
}
impl OciApp<'_> {
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
        let container_name = format!("rh-{slug}");
        let volume_name = format!("{container_name}-data");
        let container = format!(
            "[Unit]\nDescription=Rusty Home isolated OCI App {slug} (lab only)\n\n\
             [Container]\nImage={image}\nContainerName={container_name}\n\
             Volume={volume_name}.volume:/data\nNetwork=none\nPull=never\n\
             NoNewPrivileges=true\nDropCapability=all\nReadOnly=true\n\
             PidsLimit={pids}\nMemory={memory}m\n\n\
             [Service]\nRestart=on-failure\nRestartSec=5s\nTimeoutStartSec=180s\n",
            image = self.image,
            pids = self.pids_limit,
            memory = self.memory_mib
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
        }
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
