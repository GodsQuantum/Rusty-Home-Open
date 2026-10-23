use std::{fs, path::PathBuf};
fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/product-app/rusty_home")
}
#[test]
fn experimental_app_declares_real_ingress_and_no_unnecessary_host_privileges() {
    let root = project();
    let yaml = fs::read_to_string(root.join("config.yaml")).unwrap();
    let config: serde_json::Value = serde_saphyr::from_str_with_options(
        &yaml,
        serde_saphyr::options! { reject_unsupported_tags: true },
    )
    .unwrap();
    assert_eq!(config["slug"], "rusty_home");
    assert_eq!(config["stage"], "experimental");
    assert_eq!(config["boot"], "manual");
    assert_eq!(config["ingress"], true);
    assert_eq!(config["ingress_port"], 8099);
    assert_eq!(config["panel_admin"], true);
    assert_eq!(config["hassio_api"], true);
    assert_eq!(config["hassio_role"], "manager");
    for name in [
        "host_network",
        "docker_api",
        "full_access",
        "homeassistant_api",
    ] {
        assert_eq!(config[name], false, "unexpected privilege: {name}");
    }
    for name in ["protected", "apparmor"] {
        assert_eq!(config[name], true);
    }
    assert_eq!(config["arch"], serde_json::json!(["amd64"]));
    assert!(
        config.get("ports").is_none(),
        "ingress App must not expose host ports"
    );
    assert!(
        config.get("map").is_none(),
        "do not mount other App credentials"
    );
    assert!(config.get("devices").is_none());
    assert!(config.get("privileged").is_none());
}
#[test]
fn runtime_image_binds_only_the_official_authenticated_supervisor_ingress() {
    let docker = fs::read_to_string(project().join("Dockerfile")).unwrap();
    assert!(docker.starts_with("# Local App container"));
    assert!(docker.contains("FROM scratch"));
    assert!(docker.contains("COPY --chmod=0444 LICENSE /usr/share/licenses/rusty-home/LICENSE"));
    let root_license =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../LICENSE"))
            .unwrap();
    assert!(root_license.starts_with("MIT License"));
    assert!(docker.contains("USER 1000:1000"));
    assert!(docker.contains("\"--ingress\""));
    assert!(docker.contains("\"--supervisor-live\""));
    assert!(docker.contains("\"0.0.0.0:8099\""));
    for forbidden in [
        "--docker",
        "--fixture",
        "--haos-snapshot",
        "HARS_P8H_QEMU_ONLY",
        "COPY --from",
    ] {
        assert!(!docker.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn frontend_endpoints_use_ingress_relative_urls_not_home_assistant_root() {
    let html =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/index.html"))
            .unwrap();
    for endpoint in ["apps", "readiness", "python-core"] {
        assert!(
            html.contains(&format!("fetch(\"api/{endpoint}\"")),
            "missing ingress-relative route: {endpoint}"
        );
        assert!(
            !html.contains(&format!("fetch(\"/api/{endpoint}\"")),
            "absolute API route would escape Home Assistant ingress: {endpoint}"
        );
    }
}
