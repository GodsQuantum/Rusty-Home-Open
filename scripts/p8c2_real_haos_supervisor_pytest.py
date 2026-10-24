
# APPENDED TO: upstream home-assistant/operating-system/tests/supervisor_test/test_supervisor.py
# Upstream setup/fixtures come from official pinned HAOS integration test harness.
# Only runs in an ephemeral hosted QEMU guest; never against sample lab host synthetic VM.
@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(720)
def test_hars_real_product_app_install_boot(shell, shell_json):
    """Actual Rusty Home product App (not a probe) in isolated official HAOS.

    No real user's Supervisor, Home Assistant VM, radios, mounts or credentials.
    Requires a static Rust control plane, protected local App with real
    Supervisor manager token and an ingress listener.
    This is NOT an authenticated human ingress or full product migration test.
    """
    from pathlib import Path as _Path
    import os as _os
    import sys as _sys
    from time import sleep as _sleep
    _sys.path.insert(0, str(_Path(_os.environ["GITHUB_WORKSPACE"]) / "scripts"))
    from p8j_supervisor_store_recovery import ensure_supervisor_local_app_version as _ensure_store
    from p8c2_serial_invocation import parse_systemd_invocation as _systemd_invocation

    app = "local_rusty_home"
    directory = "/mnt/data/supervisor/apps/local/rusty_home"
    shell.run_check("mkdir -p " + directory)
    for asset in ("config.yaml", "Dockerfile", "LICENSE", "rusty-home"):
        shell.run_check(
            "curl --fail --silent --show-error --max-time 90 "
            "http://10.0.2.2:18673/rusty_home/" + asset
            + " -o " + directory + "/" + asset,
            timeout=105,
        )
    shell.run_check("chmod 0555 " + directory + "/rusty-home")
    installed = False
    try:
        data = _ensure_store(shell_json, app, "0.1.0-alpha.1", sleeper=_sleep)
        assert data.get("slug") == app, "Real HAOS failed to discover Rusty Home"
        assert data.get("hassio_api") is True, "Supervisor token missing"
        assert data.get("hassio_role") == "manager", "Wrong Supervisor role"
        assert data.get("ingress") is True, "Official Supervisor ingress missing"
        # Older Supervisor info serializers may omit the configured port.
        # The live HTTP 403 probe below independently verifies port 8099.
        if "ingress_port" in data:
            assert data["ingress_port"] == 8099, "Wrong ingress listener port"
        for denied in ("full_access", "docker_api", "host_network"):
            assert data.get(denied) is not True, "Excessive App privilege: " + denied

        result = shell_json(
            "ha apps install " + app + " --no-progress --raw-json", timeout=300
        )
        assert result.get("result") == "ok", "Real Rusty Home App install failed"
        installed = True
        result = shell_json(
            "ha apps start " + app + " --no-progress --raw-json", timeout=180
        )
        assert result.get("result") == "ok", "Real Rusty Home App start failed"
        data = shell_json("ha apps info " + app + " --no-progress --raw-json", timeout=60).get("data") or {}
        assert data.get("state") == "started", "Actual App process did not stay running"
        assert data.get("protected") is True, "Protected mode disappeared on install"
        assert data.get("hassio_role") == "manager", "Privilege mismatch at runtime"

        # Verify the actual App runs as non-root without dumping env or tokens.
        docker_user = shell.run_check(
            "docker inspect -f '{{.Config.User}}' app_local_rusty_home",
            timeout=30,
        )
        assert "1000:1000" in "".join(docker_user), "Actual App is not UID 1000"
        # Direct access, even with a forged admin identity header, must fail
        # because the source is NOT the official 172.30.32.2 ingress proxy.
        ip = "".join(shell.run_check(
            "docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}' "
            "app_local_rusty_home",
            timeout=30,
        )).strip().split()
        assert ip, "Real App lacks its isolated Docker network"
        status = shell.run_check(
            "curl --silent --output /dev/null --write-out '%{http_code}' "
            "--connect-timeout 3 --max-time 8 "
            "-H 'X-Remote-User-Id: claimed_admin' http://"
            + ip[0] + ":8099/health",
            timeout=20,
        )
        assert "".join(status).strip() == "403", "Untrusted direct ingress accepted"

        # Validate an actual Supervisor-driven App restart, not merely a
        # second status read or a synthetic process health endpoint.
        before_start = "".join(shell.run_check(
            "docker inspect -f '{{.State.StartedAt}}' app_local_rusty_home",
            timeout=30,
        )).strip()
        assert before_start and before_start != "<no value>", "Cannot attest initial container start"
        restarted = shell_json(
            "ha apps restart " + app + " --no-progress --raw-json",
            timeout=180,
        )
        assert restarted.get("result") == "ok", "Supervisor failed actual product App restart"
        after_start = None
        for _ in range(20):
            current = shell_json(
                "ha apps info " + app + " --no-progress --raw-json",
                timeout=45,
            )
            if current.get("result") == "ok":
                info = current.get("data") or {}
                if info.get("state") == "started":
                    times = shell.run_check(
                        "docker inspect -f '{{.State.StartedAt}}' app_local_rusty_home",
                        timeout=30,
                    )
                    after_start = "".join(times).strip()
                    if after_start and after_start != before_start:
                        break
            _sleep(2)
        assert after_start and after_start != before_start, (
            "Supervisor returned success but the actual container did not restart"
        )
        after = shell_json(
            "ha apps info " + app + " --no-progress --raw-json",
            timeout=45,
        ).get("data") or {}
        assert after.get("state") == "started", "Product failed to recover after restart"
        assert after.get("protected") is True, "Restart discarded protected mode"
        assert after.get("hassio_role") == "manager", "Restart changed Supervisor role"
        after_uid = "".join(shell.run_check(
            "docker inspect -f '{{.Config.User}}' app_local_rusty_home",
            timeout=30,
        )).strip()
        assert after_uid == "1000:1000", "Restart discarded non-root identity"
        current_ips = "".join(shell.run_check(
            "docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}' "
            "app_local_rusty_home",
            timeout=30,
        )).strip().split()
        assert current_ips, "Product App network missing after actual restart"
        for route in ("health", "api/apps", "api/store",  "api/session", "api/session/core", "api/store/apps/core_ssh/operations", "api/readiness"):
            denial = shell.run_check(
                "curl --silent --output /dev/null --write-out '%{http_code}' "
                "--connect-timeout 3 --max-time 8 "
                "-H 'X-Remote-User-Id: claimed_admin' http://"
                + current_ips[0] + ":8099/" + route,
                timeout=20,
            )
            assert "".join(denial).strip() == "403", (
                "Restart bypassed ingress authorization at " + route
            )
        # Distinct durability test: HAOS Supervisor daemon itself is restarted
        # while the real Rusty Home App remains installed and running. The App
        # must preserve its security boundaries and be discoverable afterwards.
        # Only the disposable official HAOS guest, never a household Supervisor.
        supervisor_unit = "haos-supervisor.service"
        initial_invocation = _systemd_invocation("".join(shell.run_check(
            "systemctl show -p InvocationID --value " + supervisor_unit,
            timeout=30,
        )))
        shell.run_check("systemctl restart haos-supervisor.service", timeout=150)
        next_invocation = _systemd_invocation("".join(shell.run_check(
            "systemctl show -p InvocationID --value " + supervisor_unit,
            timeout=30,
        )))
        assert next_invocation and next_invocation != initial_invocation, (
            "Supervisor restart was not independently evidenced by systemd"
        )
        recovered = None
        for _ in range(45):
            try:
                available = shell_json(
                    "ha supervisor info --no-progress --raw-json", timeout=20
                )
                if available.get("result") != "ok":
                    _sleep(2)
                    continue
                response = shell_json(
                    "ha apps info " + app + " --no-progress --raw-json",
                    timeout=20,
                )
                if response.get("result") == "ok":
                    data = response.get("data") or {}
                    if data.get("state") == "started":
                        recovered = data
                        break
            except Exception:
                # The official CLI may temporarily fail while Supervisor
                # reconnects; a bounded retry never turns that into success.
                pass
            _sleep(2)
        assert recovered is not None, (
            "Rusty Home App failed to recover through actual Supervisor daemon restart"
        )
        assert recovered.get("protected") is True, (
            "Supervisor daemon restart discarded App protected mode"
        )
        assert recovered.get("hassio_role") == "manager", (
            "Supervisor daemon restart changed the App permission role"
        )
        state_user = "".join(shell.run_check(
            "docker inspect -f '{{.Config.User}}|{{.State.Running}}' "
            "app_local_rusty_home",
            timeout=30,
        )).strip()
        assert state_user == "1000:1000|true", (
            "Supervisor restart left the real product App down or privileged"
        )
        post_ips = "".join(shell.run_check(
            "docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}' "
            "app_local_rusty_home",
            timeout=30,
        )).strip().split()
        assert post_ips, "Supervisor restart removed App private network"
        for route in ("health", "api/apps", "api/store", "api/session", "api/session/core", "api/readiness"):
            denied = shell.run_check(
                "curl --silent --output /dev/null --write-out '%{http_code}' "
                "--connect-timeout 3 --max-time 8 "
                "-H 'X-Remote-User-Id: forged_admin' http://"
                + post_ips[0] + ":8099/" + route,
                timeout=20,
            )
            assert "".join(denied).strip() == "403", (
                "Supervisor daemon restart bypassed ingress security at " + route
            )
        print(
            "REAL_RUSTY_HOME_SUPERVISOR_DAEMON_RESTART_PASS: "
            "systemd new invocation, App running nonroot, mode protected, "
            "Supervisor manager role unchanged, forged user denied"
        )
        print("REAL_RUSTY_HOME_RESTART_HAOS_PASS: real container restarted, nonroot, protected, forged ingress denied")
        print("REAL_RUSTY_HOME_PRODUCT_HAOS_PASS: installed, started, nonroot, manager, ingress deny")
    finally:
        if installed:
            cleanup = shell_json(
                "ha apps uninstall " + app + " --no-progress --raw-json",
                timeout=150,
            )
            assert cleanup.get("result") == "ok", "Disposable product App cleanup failed"


@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(480)
def test_hars_real_supervisor_store(shell, shell_json):
    """Exercise actual HAOS Supervisor store with a previously unseen public repository."""
    import json as _json
    import os as _os
    from pathlib import Path as _Path
    from subprocess import run as _run

    import sys as _sys
    _sys.path.insert(0, str(_Path(_os.environ["GITHUB_WORKSPACE"]) / "scripts"))
    from p8c2_store_codec import decode_chunked_store as _decode_store_lines

    def _real_store_over_bounded_serial():
        # HAOS host contains jq but not gzip; never send >20 KiB raw JSON lines.
        # Keep full genuine Supervisor response in a disposable guest temp file.
        path = "/mnt/data/hars-supervisor-real-store.json"
        shell.run_check(
            "ha store info --no-progress --raw-json > " + path,
            timeout=120,
        )
        try:
            output=shell.run_check(
                "jq -nr --rawfile d " + path + " '$d | length'",
                timeout=30,
            )
            declared=int("".join(output))
            if not 0 < declared <= 8 * 1024 * 1024:
                raise ValueError("Supervisor catalog exceeds hard data limit")
            chunks=[]
            for offset in range(0,declared,2048):
                end=offset+2048
                command=(
                    "jq -nr --rawfile d " + path
                    + f" '$d[{offset}:{end}] | @base64'"
                )
                chunks.append("".join(shell.run_check(command,timeout=30)))
            return _decode_store_lines(chunks,declared)
        finally:
            shell.run_check("rm -f " + path)

    store_before = _real_store_over_bounded_serial()
    assert store_before.get("result") == "ok", "The real Supervisor store CLI is not ready"
    data_before = store_before.get("data", {})
    assert isinstance(data_before.get("addons"), list), "Supervisor V1 catalogue absent"
    assert isinstance(data_before.get("repositories"), list), "Supervisor repo registry absent"
    assert data_before["addons"], "Empty real HAOS Supervisor catalogue"

    new_repository = "https://github.com/zigbee2mqtt/hassio-zigbee2mqtt"
    # This writes ONLY to the disposable QEMU snapshot; does not install an App.
    add = shell_json(
        f"ha store add {new_repository} --no-progress --raw-json",
        timeout=180,
    )
    assert add.get("result") == "ok", "Real Supervisor refused the third-party repository"

    # Supervisor refreshes its own Git repository metadata (no custom Rust parser).
    store_after = _real_store_over_bounded_serial()
    assert store_after.get("result") == "ok"
    data = store_after.get("data", {})
    repos = data.get("repositories", [])
    apps = data.get("addons", [])
    third_party = [
        r for r in repos
        if r.get("source") == new_repository or r.get("url") == new_repository
    ]
    assert third_party, "Third-party Git repository not present in actual Supervisor catalogue"
    slug = third_party[0]["slug"]
    assert any(a.get("repository") == slug for a in apps), (
        "Real Supervisor did not discover app packages inside the new repository"
    )
    assert len(apps) >= len(data_before["addons"])
    assert len(repos) > len(data_before["repositories"])

    binary = _Path(_os.environ["HARS_SUPERVISOR_PARSER"])
    result = _run(
        [str(binary)], input=_json.dumps(store_after),
        text=True, capture_output=True, timeout=15, check=False,
    )
    assert result.returncode == 0, "Our Rust parser could not read real Supervisor V1 response"
    parsed = _json.loads(result.stdout)
    assert parsed["api_version"] == "V1", "Unexpected API dialect for official HAOS"
    assert len(parsed["apps"]) == len(apps)
    assert any(a["repository"] == slug for a in parsed["apps"])
    assert parsed["official_supervisor_owner"] is True
    assert parsed["activation_authorized"] is False
    # Never log raw stores, URLs with credentials, tokens or VM screen logs.
    print(
        f"PASS P8c2 ACTUAL HAOS Supervisor {len(repos)} repos; "
        f"{len(apps)} Apps including third party; "
        "Rust real-store contract pass; install/start NOT claimed"
    )


@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(120)
def test_hars_rust_supervisor_direct(shell):
    """Run real static Rust inside official Supervisor CLI container.

    The CLI's own disposable-guest supervisor token is inherited in-process;
    no token is read or copied into the test runner or logs. This demonstrates
    direct Rust HTTP to real Supervisor, NOT a narrowly scoped app role.
    """
    shell.run_check(
        "curl --fail --silent --show-error --max-time 30 "
        "http://10.0.2.2:18673/guest_supervisor_live "
        "-o /mnt/data/hars-supervisor-live "
        "&& chmod 0700 /mnt/data/hars-supervisor-live"
    )
    shell.run_check("docker cp /mnt/data/hars-supervisor-live hassio_cli:/tmp/hars-supervisor-live")
    shell.run_check("rm -f /mnt/data/hars-supervisor-live", timeout=30)
    # Guard that the guest container itself has a legitimate Supervisor token,
    # without printing any secret to shell logs or CI.
    shell.run_check("docker exec hassio_cli sh -c 'test -n \"$SUPERVISOR_TOKEN\"'")
    lines=shell.run_check("docker exec hassio_cli /tmp/hars-supervisor-live")
    sanitized="\n".join(lines)
    assert "P8E_REAL_RUST_SUPERVISOR_AUTH_PASS" in sanitized, (
        "Rust failed its real authenticated Supervisor REST read"
    )
    assert "readonly=true" in sanitized
    assert "repos=" in sanitized and "apps=" in sanitized
    assert "jobs=" in sanitized and "jobs_readonly=true" in sanitized, (
        "Rust failed its real authenticated Supervisor GET /jobs/info"
    )
    print("P8E_GUEST_PROOF Rust authenticated HTTP GET /store and /jobs/info succeeded in genuine Supervisor CLI container")


@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(840)
def test_hars_manager_role_app(shell, shell_json):
    """True Supervisor local App: Docker FROM scratch, manager-role own token.

    This installs, starts, reads logs and uninstalls ONLY in the disposable
    HAOS QEMU snapshot. Never exposes raw Supervisor/App credentials.
    """
    from time import sleep as _sleep
    import os as _os
    import sys as _sys
    from pathlib import Path as _Path
    _sys.path.insert(0,str(_Path(_os.environ["GITHUB_WORKSPACE"])/"scripts"))
    from p8j_supervisor_store_recovery import ensure_supervisor_local_app_version as _ensure_store
    # Diagnose ENOSPC before any further local App installations/backups.
    # Free space comes ONLY from this disposable HAOS QEMU guest.
    for mount, minimum_kib in (("/mnt/data", 8 * 1024 * 1024),
                               ("/tmp", 2 * 1024)):
        rows = shell.run_check("df -Pk " + mount, timeout=30)
        fields = rows[-1].split()
        assert len(fields) >= 5 and fields[3].isdigit(), (
            "No trustworthy guest filesystem capacity for " + mount
        )
        free_kib = int(fields[3])
        print("DISPOSABLE_HAOS_FREE_KIB", mount, free_kib)
        assert free_kib >= minimum_kib, (
            "Disposable HAOS guest insufficient free storage: " + mount
        )
    app="local_hars_rust_probe"
    directory="/mnt/data/supervisor/apps/local/hars_rust_probe"
    shell.run_check("mkdir -p "+directory+"/audit_layer")
    for asset in ("config.yaml", "Dockerfile", "guest_supervisor_live", "guest_supervisor_lab_lifecycle", "guest_supervisor_managed_preview", "guest_supervisor_verified_async_install", "audit_layer/.seed"):
        shell.run_check(
            "curl --fail --silent --show-error --max-time 30 "
            "http://10.0.2.2:18673/hars_rust_probe/"+asset+
            " -o "+directory+"/"+asset,
            timeout=45,
        )
    try:
        data=_ensure_store(shell_json,app,"0.1.0",sleeper=_sleep)
        assert data.get("slug")==app, "New App not discovered by original Store"
        assert data.get("hassio_role")=="manager", "Wrong delegated API role"
        assert data.get("hassio_api") is True, "Supervisor REST access missing"
        # Before install, Supervisor legitimately reports protected=null.
        # Never equate uninstalled metadata with effective Docker isolation.
        assert data.get("protected") in (None, True), "Unexpected preinstall protection state"
        for disabled in ("docker_api","full_access","host_network"):
            assert data.get(disabled) is not True, "Unexpected extra privilege "+disabled
        result=shell_json(
            "ha apps install "+app+" --no-progress --raw-json",timeout=180
        )
        assert result.get("result")=="ok", "Real Supervisor App Docker build/install failed"
        installed=shell_json("ha apps info "+app+" --no-progress --raw-json", timeout=90).get("data") or {}
        assert installed.get("protected") is True, "Installed App lost protected security setting"
        assert installed.get("hassio_role")=="manager", "Installed App lost manager permission"
        result=shell_json(
            "ha apps start "+app+" --no-progress --raw-json",timeout=120
        )
        if result.get("result")!="ok":
            import re as _re
            reason=str(result.get("message") or "no supervisor message")
            if any(k in reason.lower() for k in ("token","bearer","authorization","password")):
                reason="[REDACTED_SENSITIVE]"
            else:
                reason=_re.sub(r"https?://\S+", "[URL]", reason)
                reason=_re.sub(r"[A-Za-z0-9_\-+/=]{72,}","[LONG_REDACTED]",reason)
                reason="".join(c for c in reason if c.isprintable())[:180]
            # Only known sanitized test marker and generic container status:
            marker=shell.run_check(
                "ha apps logs "+app+" --no-progress | "
                "grep -E 'P8E_REAL_RUST_SUPERVISOR_AUTH_PASS|P8E: denied' || true",
                timeout=30,
            )
            states=shell.run_check(
                "docker ps -a --format '{{.Names}}|{{.Status}}' | "
                "grep 'hars_rust_probe' || true",
                timeout=30,
            )
            marker_state=("success" if any("P8E_REAL_RUST_SUPERVISOR_AUTH_PASS" in m for m in marker)
                else "denied" if any("P8E: denied" in m for m in marker)
                else "not-seen")
            container_state="; ".join(states)[:180] if states else "no-container"
            raise AssertionError(
                "Real Supervisor App start failed; reason="+reason
                +"; probe="+marker_state+"; docker="+container_state
            )
        running=shell_json("ha apps info "+app+" --no-progress --raw-json",timeout=90).get("data") or {}
        assert running.get("state")=="started", "Supervisor App is not persistently started"
        # Supervised static proof stays alive after GET. Consult only its sanitized
        # success marker, never dump arbitrary App logs or credentials.
        for _ in range(25):
            log_lines=shell.run_check(
                "ha apps logs "+app+" --no-progress | "
                "grep 'P8E_REAL_RUST_SUPERVISOR_AUTH_PASS' || true",
                timeout=30,
            )
            if any("P8E_REAL_RUST_SUPERVISOR_AUTH_PASS" in s for s in log_lines):
                print("P8F_TRUE_SUPERVISOR_APP_PASS manager role, protected, no Docker privileges")
                break
            _sleep(2)
        else:
            raise AssertionError("Rust manager-role App never authenticated to Supervisor")
        # P8N: the real manager-role Rust App submits ONE background
        # Supervisor Store installation, polls its authentic job and installed
        # inventory independently, and only then releases the durable fence.
        # Entirely disposable on this QEMU snapshot; no household host/device.
        verified_slug = "local_hars_verified_fixture"
        verified_dir = "/mnt/data/supervisor/apps/local/hars_verified_fixture"
        shell.run_check("mkdir -p " + verified_dir)
        verified_installed = False
        try:
            for asset in ("config.yaml", "Dockerfile", "guest_supervisor_live"):
                shell.run_check(
                    "curl --fail --silent --show-error --max-time 30 "
                    "http://10.0.2.2:18673/hars_verified_fixture/" + asset
                    + " -o " + verified_dir + "/" + asset,
                    timeout=45,
                )
            candidate = _ensure_store(shell_json, verified_slug, "1.0.0", sleeper=_sleep)
            assert candidate.get("slug") == verified_slug
            assert candidate.get("available") is not False
            assert candidate.get("full_access") is not True
            # No HTTP client retries this POST. The Rust binary owns the
            # audited permit, DB fence, typed job binding and verified release.
            try:
                records = shell.run_check(
                    "docker exec app_local_hars_rust_probe "
                    "/hars-fenced-install --acknowledge-disposable-qemu "
                    + verified_slug,
                    timeout=230,
                )
            except Exception:
                # Do not swallow an actual failed install proof. This
                # independent official GET reports only a safe, finite
                # installed flag/state and NEVER raw Supervisor messages.
                try:
                    observed = shell_json(
                        "ha apps info " + verified_slug + " --no-progress --raw-json",
                        timeout=50,
                    )
                    info = observed.get("data") or {}
                    state = info.get("state")
                    if state not in ("started", "stopped", "unknown", "error"):
                        state = "other"
                    # Current official Supervisor info does not always expose
                    # an "installed" boolean; a matching real installed
                    # version and managed-App state are stronger evidence.
                    verified_installed = (
                        info.get("slug") == verified_slug
                        and info.get("version") == "1.0.0"
                        and state in ("started", "stopped", "error")
                    )
                    print("P8N_GUEST_OFFICIAL_VERSION_AND_STATE",
                          int(verified_installed), "STATE", state)
                except Exception:
                    print("P8N_GUEST_OFFICIAL_READ_UNAVAILABLE")
                raise
            verified_closure = any(
                "P8N_REAL_SUPERVISOR_ASYNC_INSTALL_JOB_APP_AND_REDB_PASS" in line
                for line in records
            )
            verified_expiry = any(
                "P8N_REAL_HAOS_JOB_EXPIRED_UNCERTAIN_LOCK_STAYS_CLOSED_PASS" in line
                for line in records
            )
            assert verified_closure ^ verified_expiry, (
                "Neither one positive job-and-App proof nor one verified "
                "fail-closed post-job-cleanup proof was observed"
            )
            result = shell_json(
                "ha apps info " + verified_slug + " --no-progress --raw-json",
                timeout=75,
            )
            installed = result.get("data") or {}
            assert result.get("result") == "ok"
            assert installed.get("slug") == verified_slug
            assert installed.get("version") == "1.0.0"
            assert installed.get("state") in ("started", "stopped")
            verified_installed = True
            if verified_closure:
                print("P8N_GENUINE_HAOS_ASYNC_INSTALL_AND_DURABLE_UNLOCK_PASS")
            else:
                # Deliberately NOT a job-success nor unlock assertion.
                # HAOS with cleanup=True evicts the finished job; Rust
                # must refuse to treat missing proof as authorisation.
                print("P8N_GENUINE_HAOS_JOB_EVICTED_SAFE_FENCE_PASS")
                print("P8N_POSITIVE_TERMINAL_JOB_PROOF_NOT_CERTIFIED")
        finally:
            # Explicitly remove only this disposable local test App. Never
            # repeat an uncertain install after a guest command timeout.
            if not verified_installed:
                try:
                    current = shell_json(
                        "ha apps info " + verified_slug + " --no-progress --raw-json",
                        timeout=45,
                    ).get("data") or {}
                    verified_installed = (
                        current.get("slug") == verified_slug
                        and current.get("version") == "1.0.0"
                        and current.get("state") in ("started", "stopped", "error")
                    )
                except Exception:
                    pass
            if verified_installed:
                removed = shell_json(
                    "ha apps uninstall " + verified_slug
                    + " --no-progress --raw-json",
                    timeout=180,
                )
                assert removed.get("result") == "ok", "QEMU fixture cleanup refused"
            shell.run_check("rm -rf " + verified_dir)

        # P8i.d.6: dedicated least-privilege Core-only App in QEMU.
        # Manager App retains homeassistant_api:false and cannot query Core.
        # No user IDs, tokens or JSON are exposed in test output.
        core_app = "local_hars_core_ws_probe"
        core_dir = "/mnt/data/supervisor/apps/local/hars_core_ws_probe"
        shell.run_check("mkdir -p " + core_dir)
        for asset in ("config.yaml", "Dockerfile", "guest_core_users_probe"):
            shell.run_check(
                "curl --fail --silent --show-error --max-time 30 "
                "http://10.0.2.2:18673/hars_core_ws_probe/" + asset +
                " -o " + core_dir + "/" + asset,
                timeout=45,
            )
        core_installed = False
        try:
            info=_ensure_store(shell_json,core_app,"0.1.0",sleeper=_sleep)
            assert info.get("slug") == core_app
            assert info.get("homeassistant_api") is True
            assert info.get("hassio_api") is not True
            assert info.get("hassio_role") != "manager"
            assert info.get("docker_api") is not True
            assert shell_json(
                "ha apps install " + core_app + " --no-progress --raw-json", timeout=180
            ).get("result") == "ok", "Core-only static App installation failed"
            core_installed = True
            assert shell_json(
                "ha apps start " + core_app + " --no-progress --raw-json", timeout=120
            ).get("result") == "ok", "Core-only static App failed to start"
            proof = shell.run_check(
                "docker exec app_local_hars_core_ws_probe /hars-core-users --probe",
                timeout=60,
            )
            # WebSocket authentication alone is NOT proof of a usable
            # authorization oracle. Current Core must actually return the
            # user inventory, not a denied config/auth/list command.
            assert any("P8I_REAL_CORE_WS_USERS_READ" in item for item in proof), (
                "Real Core WS did not provide authorized user inventory"
            )
            assert all(
                "P8I_REAL_CORE_WS_LIST_DENIED" not in item for item in proof
            ), "P8I_REAL_CORE_WS_PERMISSION_DENIED: no RBAC authority"
            print("P8I_REAL_CORE_WS_AUTHENTICATED_READ_ONLY_PROBE_PASS")
        finally:
            try:
                if core_installed:
                    uninstall = shell_json(
                        "ha apps uninstall " + core_app + " --no-progress --raw-json",
                        timeout=120,
                    )
                    assert uninstall.get("result") == "ok", (
                        "Disposable Core-only App cleanup failed"
                    )
            finally:
                shell.run_check("rm -rf " + core_dir)
        # P8g: only the core_ssh writable fixture is allowed, running on
        # the disposable QEMU snapshot. Rust rather than CLI stops/starts it.
        installed_by_rust = shell.run_check(
            "docker exec app_local_hars_rust_probe "
            "/hars-manager --acknowledge-disposable-qemu install core_ssh",
            timeout=180,
        )
        assert any("P8I_QEMU_FIXTURE_SESSION_GATE_PASS" in m for m in installed_by_rust), (
            "Disposable HAOS Rust guest did not pass session registry and one-time confirmation"
        )
        assert any("P8J_REAL_INSTALL_AVAILABILITY_PREFLIGHT_PASS" in m for m in installed_by_rust), (
            "Rust did not prove official Supervisor installation availability before POST"
        )
        assert any("P8H_REAL_SUPERVISOR_INSTALL_PASS" in m for m in installed_by_rust), (
            "Validated generic Rust API did not install fixture core_ssh"
        )
        assert (shell_json(
            "ha apps info core_ssh --no-progress --raw-json", timeout=60
        ).get("data") or {}).get("version"), "Rust-installed fixture missing"
        # P8j.d: interrogate the actual Supervisor App update eligibility
        # without sending any UPDATE POST. Upstream ignores version suffixes.
        installed_update_info = (shell_json(
            "ha apps info core_ssh --no-progress --raw-json", timeout=60
        ).get("data") or {})
        update_probe = shell.run_check(
            "docker exec app_local_hars_rust_probe "
            "/hars-manager --acknowledge-disposable-qemu probe-update core_ssh",
            timeout=60,
        )
        expected_probe = (
            "P8JD_REAL_SUPERVISOR_UPDATE_READY"
            if installed_update_info.get("update_available") is True
            and isinstance(installed_update_info.get("version"), str)
            and isinstance(installed_update_info.get("version_latest"), str)
            and installed_update_info["version"] != installed_update_info["version_latest"]
            else "P8JD_REAL_SUPERVISOR_UPDATE_DENIED"
        )
        assert any(expected_probe in line for line in update_probe), (
            "Rust did not enforce real Supervisor installed-App update eligibility"
        )
        print("P8JD_REAL_READ_ONLY_UPDATE_PREFLIGHT_PASS")
        try:
            assert shell_json(
                "ha apps start core_ssh --no-progress --raw-json", timeout=120
            ).get("result") == "ok", "Disposable fixture SSH start failed"
            assert (shell_json(
                "ha apps info core_ssh --no-progress --raw-json", timeout=60
            ).get("data") or {}).get("state") == "started"
            stopped = shell.run_check(
                "docker exec app_local_hars_rust_probe /hars-lifecycle stop",
                timeout=45,
            )
            assert any("P8G_REAL_RUST_MANAGER_STOP_PASS" in line for line in stopped), (
                "Genuine manager-role Rust never authenticated STOP command"
            )
            stop_state = (shell_json(
                "ha apps info core_ssh --no-progress --raw-json", timeout=60
            ).get("data") or {}).get("state")
            # Supervisor upstream issue #6840: a manual SIGTERM exit 143
            # can report state=error even after a successful STOP. Do not
            # mistake that for a running container: check Docker directly.
            assert stop_state in ("stopped", "error"), (
                "Unexpected Supervisor state after Rust STOP: " + str(stop_state)
            )
            live_containers = shell.run_check(
                "docker ps --format '{{.Names}}' | "
                "grep -E '(^addon_|^app_)core_ssh$' || true",
                timeout=30,
            )
            assert not live_containers, (
                "core_ssh still running after authenticated Rust STOP"
            )
            print("P8H_STOP_CONFIRMED_NOT_RUNNING")
            started = shell.run_check(
                "docker exec app_local_hars_rust_probe /hars-lifecycle start",
                timeout=60,
            )
            assert any("P8G_REAL_RUST_MANAGER_START_PASS" in line for line in started), (
                "Genuine manager-role Rust never authenticated START command"
            )
            assert (shell_json(
                "ha apps info core_ssh --no-progress --raw-json", timeout=60
            ).get("data") or {}).get("state")=="started", (
                "Supervisor did not reflect Rust START"
            )
            print("P8G_REAL_RUST_MANAGER_STOP_START_BOTH_VERIFIED")
            restarted_by_rust = shell.run_check(
                "docker exec app_local_hars_rust_probe "
                "/hars-manager --acknowledge-disposable-qemu restart core_ssh",
                timeout=180,
            )
            assert any("P8H_REAL_SUPERVISOR_RESTART_PASS" in m for m in restarted_by_rust), (
                "Generic Rust restart did not return Supervisor success"
            )
            assert (shell_json(
                "ha apps info core_ssh --no-progress --raw-json", timeout=60
            ).get("data") or {}).get("state")=="started", "Restart state not verified"
            # P8j.c: inspect real disposable core_ssh /data, not just metadata.
            containers = shell.run_check(
                "docker ps --format '{{.Names}}' | "
                "grep -E '^(app_|addon_)core_ssh$' || true", timeout=30
            )
            assert len(containers)==1, "Expected exactly one running core_ssh fixture"
            core_container = containers[0].strip()
            payload_path = "/data/hars_p8j_roundtrip_fixture"
            shell.run_check(
                "docker exec "+core_container+
                " sh -c 'echo P8J_DATA_BYTES_ORIGINAL_20261020 > "+payload_path+"'",
                timeout=30,
            )
            original_data = shell.run_check(
                "docker exec "+core_container+" cat "+payload_path, timeout=30
            )
            assert any(x.strip()=="P8J_DATA_BYTES_ORIGINAL_20261020" for x in original_data), (
                "Unable to write known bytes into actual App data volume"
            )
            partial = shell.run_check(
                "docker exec app_local_hars_rust_probe "
                "/hars-manager --acknowledge-disposable-qemu backup-partial core_ssh",
                timeout=180,
            )
            backup_slugs = [m.split("=",1)[1].strip() for m in partial if m.startswith("P8H_BACKUP_SLUG=")]
            assert len(backup_slugs)==1, "Generic Rust partial backup did not produce a known backup ID"
            backup_slug = backup_slugs[0]
            backup_reply = shell_json(
                "ha backups info "+backup_slug+" --no-progress --raw-json", timeout=120
            )
            assert backup_reply.get("result")=="ok", "Supervisor does not recognize Rust backup"
            backup_details = backup_reply.get("data") or {}
            assert backup_details.get("slug")==backup_slug, "Supervisor backup slug mismatch"
            assert backup_details.get("protected") is False, "Unexpected protected backup"
            addon_entries = [x for x in backup_details.get("addons", [])
                             if isinstance(x, dict) and x.get("slug")=="core_ssh"]
            assert len(addon_entries)==1, "Target App missing or duplicated in backup"
            expected_backup_version = addon_entries[0].get("version")
            assert isinstance(expected_backup_version, str) and expected_backup_version, "Backup has no App version"
            # Corrupt fixture /data after backup, BEFORE Rust restoration.
            shell.run_check(
                "docker exec "+core_container+
                " sh -c 'echo P8J_DATA_BYTES_CORRUPTED_20261020 > "+payload_path+"'",
                timeout=30,
            )
            corrupt_data = shell.run_check(
                "docker exec "+core_container+" cat "+payload_path, timeout=30
            )
            assert corrupt_data and any(x.strip()=="P8J_DATA_BYTES_CORRUPTED_20261020" for x in corrupt_data), (
                "Mutation of known App data did not take effect"
            )
            restored = shell.run_check(
                "docker exec app_local_hars_rust_probe "
                "/hars-manager --acknowledge-disposable-qemu restore-partial core_ssh "
                +backup_slug,
                timeout=240,
            )
            assert any("P8H_REAL_SUPERVISOR_RESTORE_PARTIAL_PASS" in m for m in restored), (
                "Generic Rust partial restore failed in disposable guest"
            )
            restored_data = (shell_json(
                "ha apps info core_ssh --no-progress --raw-json", timeout=90
            ).get("data") or {})
            assert restored_data.get("state")=="started", "Restored core_ssh not started"
            assert restored_data.get("version")==expected_backup_version, (
                "Restored App version differs from backup metadata"
            )
            recovered_data = shell.run_check(
                "docker exec "+core_container+" cat "+payload_path, timeout=30
            )
            assert recovered_data and any(
                x.strip()=="P8J_DATA_BYTES_ORIGINAL_20261020" for x in recovered_data
            ), "Real App /data bytes did not return to pre-backup content"

            # P8j.e: genuine two-version local App on QEMU's SNAPSHOT disk.
            # This is a separate fixture; no real household App is modified.
            version_app = "local_hars_version_fixture"
            version_dir = "/mnt/data/supervisor/apps/local/hars_version_fixture"
            shell.run_check("mkdir -p "+version_dir)
            try:
                for asset in ("config.yaml", "Dockerfile", "guest_supervisor_live"):
                    shell.run_check(
                        "curl --fail --silent --show-error --max-time 30 "
                        "http://10.0.2.2:18673/hars_version_fixture/"+asset+
                        " -o "+version_dir+"/"+asset,
                        timeout=45,
                    )
                v1_store=_ensure_store(shell_json,version_app,"1.0.0",sleeper=_sleep)
                assert v1_store.get("version_latest")=="1.0.0", "Fixture v1 not in Supervisor Store"
                installed_v1 = shell.run_check(
                    "docker exec app_local_hars_rust_probe "
                    "/hars-manager --acknowledge-disposable-qemu install "+version_app,
                    timeout=180,
                )
                assert any("P8H_REAL_SUPERVISOR_INSTALL_PASS" in x for x in installed_v1)
                assert shell_json(
                    "ha apps start "+version_app+" --no-progress --raw-json", timeout=120
                ).get("result")=="ok", "Versioned fixture v1 did not start"
                assert (shell_json(
                    "ha apps info "+version_app+" --no-progress --raw-json", timeout=60
                ).get("data") or {}).get("version")=="1.0.0"
                # The only writable file belongs to the ephemeral fixture
                # itself, discovered from Docker /data bind in this QEMU guest.
                source = shell.run_check(
                    "docker inspect --format "
                    "'{{range .Mounts}}{{if eq .Destination \"/data\"}}{{.Source}}{{end}}{{end}}' "
                    "app_"+version_app, timeout=30
                )
                assert len(source)==1
                mount = source[0].strip()
                assert mount.startswith("/mnt/data/supervisor/") and mount.endswith("/"+version_app)
                assert " " not in mount and "'" not in mount and ".." not in mount
                fname = mount+"/hars_v1_marker"
                shell.run_check(
                    "sh -c 'echo P8JE_VERSION_ONE_ORIGINAL_BYTES > "+fname+"'"
                )
                import hashlib as _hashlib
                expected_v1_hash = _hashlib.sha256(b"P8JE_VERSION_ONE_ORIGINAL_BYTES\n").hexdigest()
                hashes = shell.run_check("sha256sum "+fname,timeout=30)
                assert any(expected_v1_hash in line for line in hashes), "V1 file hash mismatch"
                b1 = shell.run_check(
                    "docker exec app_local_hars_rust_probe "
                    "/hars-manager --acknowledge-disposable-qemu backup-partial "+version_app,
                    timeout=180,
                )
                backup_ids = [x.split("=",1)[1].strip() for x in b1 if x.startswith("P8H_BACKUP_SLUG=")]
                assert len(backup_ids)==1, "No real prior-version backup"
                old_backup = backup_ids[0]
                binfo = shell_json(
                    "ha backups info "+old_backup+" --no-progress --raw-json",timeout=120
                ).get("data") or {}
                assert any(x.get("slug")==version_app and x.get("version")=="1.0.0"
                           for x in binfo.get("addons",[]) if isinstance(x,dict)), (
                    "Supervisor partial backup did not capture App v1"
                )
                shell.run_check(
                    "sed -i 's/version: \"1.0.0\"/version: \"1.0.1\"/' "
                    +version_dir+"/config.yaml",
                    timeout=30,
                )
                shell.run_check(
                    "sed -i 's/io.hass.version=\"1.0.0\"/io.hass.version=\"1.0.1\"/' "
                    +version_dir+"/Dockerfile",
                    timeout=30,
                )
                candidates=_ensure_store(shell_json,version_app,"1.0.1",sleeper=_sleep)
                assert candidates.get("version")=="1.0.0"
                assert candidates.get("version_latest")=="1.0.1"
                assert candidates.get("update_available") is True, (
                    "Supervisor does not advertise a real version change"
                )
                update_result = shell.run_check(
                    "docker exec app_local_hars_rust_probe "
                    "/hars-manager --acknowledge-disposable-qemu update-with-backup "+version_app,
                    timeout=240,
                )
                assert any("P8H_REAL_SUPERVISOR_UPDATE_WITH_BACKUP_PASS" in x
                           for x in update_result), "Rust V1->V2 backed update was not confirmed"
                after_update = shell_json(
                    "ha apps info "+version_app+" --no-progress --raw-json", timeout=90
                ).get("data") or {}
                assert after_update.get("version")=="1.0.1", "No actual V2 installation"
                shell.run_check(
                    "sh -c 'echo P8JE_VERSION_TWO_CHANGED_BYTES > "+fname+"'"
                )
                hashes2=shell.run_check("sha256sum "+fname,timeout=30)
                assert not any(expected_v1_hash in line for line in hashes2), (
                    "V2 mutation did not actually change data"
                )
                # P8j.f: deliberately SIGKILL only the disposable Rust
                # manager App after V2 update/data mutation, before rollback.
                # Supervisor backup must survive losing its client process.
                killed = shell.run_check(
                    "docker kill --signal=KILL app_local_hars_rust_probe",
                    timeout=30,
                )
                assert any("app_local_hars_rust_probe" in line for line in killed), (
                    "QEMU Manager process did not receive SIGKILL"
                )
                recovered_manager = False
                for _ in range(8):
                    try:
                        _start = shell_json(
                            "ha apps start local_hars_rust_probe --no-progress --raw-json",
                            timeout=90,
                        )
                        if _start.get("result") == "ok":
                            up = shell.run_check(
                                "docker ps --format '{{.Names}}' | "
                                "grep '^app_local_hars_rust_probe$' || true", timeout=30
                            )
                            if any(line.strip()=="app_local_hars_rust_probe" for line in up):
                                recovered_manager = True
                                break
                    except Exception:
                        pass
                    _sleep(2)
                assert recovered_manager, (
                    "Disposable Rust Manager App did not restart after SIGKILL"
                )
                print("P8JF_MANAGER_KILL9_VERIFIED")
                rolled = shell.run_check(
                    "docker exec app_local_hars_rust_probe "
                    "/hars-manager --acknowledge-disposable-qemu restore-partial "
                    +version_app+" "+old_backup,
                    timeout=240,
                )
                assert any("P8H_REAL_SUPERVISOR_RESTORE_PARTIAL_PASS" in x
                           for x in rolled), "Rust did not restore the V1 backup"
                rolled_info = shell_json(
                    "ha apps info "+version_app+" --no-progress --raw-json",timeout=90
                ).get("data") or {}
                assert rolled_info.get("version")=="1.0.0", (
                    "App did not actually roll back its installed version to V1"
                )
                hashes3=shell.run_check("sha256sum "+fname, timeout=30)
                assert hashes3 and any(expected_v1_hash in line for line in hashes3), (
                    "V1 App data was not restored byte for byte"
                )
                print("P8JE_REAL_TWO_VERSION_ROLLBACK_PASS")
            finally:
                # Only the explicitly introduced QEMU fixture is removed.
                from time import sleep as _v_sleep
                try:
                    current = shell_json(
                        "ha apps info "+version_app+" --no-progress --raw-json", timeout=40
                    ).get("data") or {}
                    if current.get("installed") is True:
                        for _ in range(8):
                            try:
                                r=shell_json(
                                    "ha apps uninstall "+version_app+" --no-progress --raw-json",
                                    timeout=90,
                                )
                                if r.get("result")=="ok": break
                            except Exception:
                                pass
                            _v_sleep(2)
                finally:
                    shell.run_check("rm -rf "+version_dir)
            print("P8J_ACTUAL_APP_DATA_RECOVERED_PASS")
            print("P8J_REAL_BACKUP_RESTORE_PREFLIGHT_PASS")
            print("P8H_REAL_RUST_INSTALL_RESTART_BACKUP_RESTORE_VERIFIED")
        finally:
            # QEMU-only cleanup must not mask the original Rust error.
            # Supervisor may still hold a transient async Docker stop job.
            import sys as _sys
            cleanup_ok = False
            for _ in range(12):
                try:
                    reply = shell_json(
                        "ha apps uninstall core_ssh --no-progress --raw-json", timeout=120
                    )
                    if reply.get("result") == "ok":
                        cleanup_ok = True
                        break
                except Exception:
                    pass
                _sleep(2)
            if not cleanup_ok:
                if _sys.exc_info()[0] is None:
                    raise AssertionError("Disposable QEMU core_ssh cleanup never succeeded")
                print("P8G: fixture cleanup incomplete in ephemeral QEMU; original failure preserved")
        assert shell_json(
            "ha apps uninstall "+app+" --no-progress --raw-json",timeout=120
        ).get("result")=="ok", "App uninstall incomplete in QEMU"
    finally:
        shell.run_check("rm -rf "+directory)


@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(540)
def test_hars_full_backup_archive_qemu(shell, shell_json):
    """Genuine full HAOS backup, created only on a disposable KVM snapshot."""
    import re as _re
    # Only write a synthetic marker in the disposable guest's SHARE folder.
    marker = "/mnt/data/supervisor/share/rusty_home_full_restore_marker"
    original = "P8JI_SHARE_BYTES_ORIGINAL_20261021"
    shell.run_check("mkdir -p /mnt/data/supervisor/share")
    shell.run_check("sh -c 'echo " + original + " > " + marker + "'")
    assert any(original in x for x in shell.run_check("cat " + marker))
    created = shell_json(
        'ha backups new --name "Rusty Home QEMU Full Backup" --no-progress --raw-json',
        timeout=330,
    )
    assert created.get("result") == "ok", "Supervisor full backup creation failed"
    data = created.get("data") or {}
    slug = data.get("slug")
    assert isinstance(slug, str) and _re.fullmatch(r"[0-9a-f]{8,16}", slug), (
        "Supervisor returned an invalid backup identifier"
    )
    details = shell_json(
        "ha backups info " + slug + " --no-progress --raw-json", timeout=90
    )
    assert details.get("result") == "ok"
    item = details.get("data") or {}
    assert item.get("slug") == slug
    assert item.get("type") == "full", "Supervisor did not make a full backup"
    assert isinstance(item.get("homeassistant"), str) and item["homeassistant"], (
        "Backup excludes Home Assistant Core"
    )
    assert isinstance(item.get("size_bytes"), int) and item["size_bytes"] > 1024
    assert isinstance(item.get("addons"), list)
    assert isinstance(item.get("folders"), list) and "share" in item["folders"], (
        "Complete backup omitted the disposable share folder marker"
    )
    actual = shell.run_check(
        "sha256sum /mnt/data/supervisor/backup/" + slug + ".tar", timeout=60
    )
    assert len(actual) == 1 and _re.fullmatch(
        r"[0-9a-f]{64}", actual[0].split()[0]
    ), "Full HAOS backup archive SHA256 could not be verified"
    print("P8JH_REAL_FULL_HAOS_BACKUP_ARCHIVE_SHA256_PASS")
    shell.run_check("sh -c 'echo P8JI_SHARE_BYTES_MUTATED > " + marker + "'")
    assert not any(original in x for x in shell.run_check("cat " + marker))
    # Only an opaque, validated slug is passed to the next QEMU test.
    shell.run_check("sh -c 'echo " + slug + " > /tmp/rusty_home_full_backup_slug'")


@pytest.mark.dependency(depends=["test_hars_full_backup_archive_qemu"])
@pytest.mark.timeout(1000)
def test_hars_full_restore_archive_qemu(shell, shell_json):
    """Restore entire disposable HAOS snapshot and verify real bytes/Core state."""
    import re as _re
    from time import sleep as _sleep
    rows = shell.run_check("cat /tmp/rusty_home_full_backup_slug", timeout=30)
    assert len(rows) == 1
    slug = rows[0].strip()
    assert _re.fullmatch(r"[0-9a-f]{8,16}", slug), "Invalid internal backup slug"
    marker = "/mnt/data/supervisor/share/rusty_home_full_restore_marker"
    original = "P8JI_SHARE_BYTES_ORIGINAL_20261021"
    assert not any(original in line for line in shell.run_check("cat " + marker)), (
        "Marker was not corrupted before requested full restoration"
    )
    backup = shell_json(
        "ha backups info " + slug + " --no-progress --raw-json", timeout=90
    ).get("data") or {}
    assert backup.get("type") == "full" and backup.get("slug") == slug
    core_version = backup.get("homeassistant")
    assert isinstance(core_version, str) and core_version, (
        "No Home Assistant Core version in the source full backup"
    )
    # Official CLI defaults to POST /backups/<slug>/restore/full when no
    # --app, --folders or --homeassistant option is supplied.
    answer = shell_json(
        "ha backups restore " + slug + " --no-progress --raw-json",
        timeout=480,
    )
    assert answer.get("result") == "ok", "Real Supervisor full restore was refused"
    observed = False
    for _ in range(60):
        try:
            content = shell.run_check("cat " + marker, timeout=30)
            supervisor = shell_json(
                "ha supervisor info --no-progress --raw-json", timeout=45
            )
            core = shell_json("ha core info --no-progress --raw-json", timeout=45)
            current = core.get("data") or {}
            if (any(x.strip() == original for x in content)
                and supervisor.get("result") == "ok"
                and core.get("result") == "ok"
                and current.get("version") == core_version):
                observed = True
                break
        except Exception:
            pass
        _sleep(5)
    assert observed, "HAOS full restore did not recover share bytes and Core identity"
    print("P8JI_REAL_FULL_HAOS_RESTORE_CONTENT_AND_CORE_PASS")


@pytest.mark.dependency(depends=["test_hars_full_restore_archive_qemu"])
@pytest.mark.timeout(1080)
def test_hars_protected_full_backup_restore_qemu(shell, shell_json):
    """Real password-protected HAOS full backup and full restore in disposable KVM.

    Test password is a public, fixed synthetic fixture; never a real credential.
    """
    import re as _re
    from time import sleep as _sleep

    marker = "/mnt/data/supervisor/share/rusty_home_protected_restore_marker"
    original = "P8JJ_PROTECTED_ORIGINAL_BYTES_20261021"
    fake_password = "RustyQemuSyntheticOnly_2026_Protected"
    shell.run_check("mkdir -p /mnt/data/supervisor/share")
    shell.run_check("sh -c 'echo " + original + " > " + marker + "'")
    assert any(line.strip() == original for line in shell.run_check("cat " + marker)), (
        "Cannot set up QEMU-only protected backup marker"
    )
    # Explicit password protects the full backup. No --app or --folders.
    created = shell_json(
        'ha backups new --name "Rusty Home Protected QEMU" '
        '--password "' + fake_password + '" --no-progress --raw-json',
        timeout=420,
    )
    assert created.get("result") == "ok", "Protected full backup creation refused"
    data = created.get("data") or {}
    slug = data.get("slug")
    assert isinstance(slug, str) and _re.fullmatch(r"[0-9a-f]{8,16}", slug), (
        "Invalid protected full backup slug"
    )
    info = shell_json(
        "ha backups info " + slug + " --no-progress --raw-json", timeout=90
    )
    assert info.get("result") == "ok", "Protected backup info unavailable"
    item = info.get("data") or {}
    assert item.get("slug") == slug
    assert item.get("type") == "full", "Protected backup was partial"
    assert item.get("protected") is True, "Backup is not reported encrypted"
    assert isinstance(item.get("homeassistant"), str) and item["homeassistant"]
    assert isinstance(item.get("folders"), list) and "share" in item["folders"]
    assert isinstance(item.get("addons"), list)
    assert isinstance(item.get("size_bytes"), int) and item["size_bytes"] > 1024
    digest = shell.run_check(
        "sha256sum /mnt/data/supervisor/backup/" + slug + ".tar", timeout=60
    )
    assert len(digest) == 1 and _re.fullmatch(
        r"[0-9a-f]{64}", digest[0].split()[0]
    ), "Protected full backup archive missing or corrupt"
    print("P8JJ_REAL_PROTECTED_FULL_BACKUP_META_AND_SHA256_PASS")

    shell.run_check("sh -c 'echo P8JJ_PROTECTED_MUTATED_BYTES > " + marker + "'")
    assert not any(line.strip() == original for line in shell.run_check("cat " + marker))
    # Official CLI: protected FULL restore, passing a synthetic QEMU-only
    # password; no --app, --folders, --homeassistant or partial restore flags.
    restored = shell_json(
        "ha backups restore " + slug + ' --password "' + fake_password
        + '" --no-progress --raw-json',
        timeout=600,
    )
    assert restored.get("result") == "ok", "Supervisor protected full restore refused"
    healthy = False
    for _ in range(60):
        try:
            content = shell.run_check("cat " + marker, timeout=30)
            supervisor = shell_json(
                "ha supervisor info --no-progress --raw-json", timeout=45
            )
            core = shell_json("ha core info --no-progress --raw-json", timeout=45)
            if (any(x.strip() == original for x in content)
                and supervisor.get("result") == "ok"
                and core.get("result") == "ok"
                and (core.get("data") or {}).get("version") == item["homeassistant"]):
                healthy = True
                break
        except Exception:
            pass
        _sleep(5)
    assert healthy, "Protected full restore did not recover bytes and real Core health"
    print("P8JJ_REAL_PROTECTED_FULL_RESTORE_EXACT_BYTES_AND_CORE_PASS")
    shell.run_check("sh -c 'echo " + slug + " > /tmp/rusty_home_protected_backup_slug'")


@pytest.mark.dependency(depends=["test_hars_protected_full_backup_restore_qemu"])
@pytest.mark.timeout(540)
def test_hars_protected_full_restore_wrong_password_rejected_qemu(shell, shell_json):
    """Real HAOS must reject a wrong encryption password without mutating share."""
    import json as _json
    import re as _re

    rows = shell.run_check("cat /tmp/rusty_home_protected_backup_slug", timeout=30)
    assert len(rows) == 1
    slug = rows[0].strip()
    assert _re.fullmatch(r"[0-9a-f]{8,16}", slug), "Invalid QEMU-only backup slug"
    backup = shell_json("ha backups info " + slug + " --no-progress --raw-json", timeout=90)
    assert backup.get("result") == "ok"
    meta = backup.get("data") or {}
    assert meta.get("slug") == slug and meta.get("type") == "full"
    assert meta.get("protected") is True
    expected_core = meta.get("homeassistant")
    assert isinstance(expected_core, str) and expected_core

    marker = "/mnt/data/supervisor/share/rusty_home_protected_restore_marker"
    mutated = "P8JK_WRONG_PASSWORD_MUST_NOT_RESTORE_20261022"
    shell.run_check("sh -c 'echo " + mutated + " > " + marker + "'")
    before = shell.run_check("sha256sum " + marker, timeout=45)
    assert len(before) == 1 and _re.fullmatch(r"[0-9a-f]{64}", before[0].split()[0])
    archive = "/mnt/data/supervisor/backup/" + slug + ".tar"
    archive_before = shell.run_check("sha256sum " + archive, timeout=45)
    assert len(archive_before) == 1

    # Capture the actual native CLI response; the synthetic wrong password
    # must yield an explicit authentication/decryption failure. A generic
    # network/timeout error is NOT acceptable evidence of rejection.
    wrong = "RustyQemuSyntheticOnly_2026_WRONG"
    result_file = "/tmp/rusty_home_wrong_password_result.json"
    status_file = "/tmp/rusty_home_wrong_password_exit.txt"
    shell.run_check(
        "sh -c 'ha backups restore " + slug
        + ' --password "' + wrong + '" --no-progress --raw-json'
        + " > " + result_file + " 2>&1; echo $? > " + status_file + "'",
        timeout=300,
    )
    lines = shell.run_check("cat " + result_file, timeout=30)
    assert lines, "No real Supervisor password rejection response"
    result = _json.loads("\n".join(lines))
    assert isinstance(result, dict) and result.get("result") != "ok", (
        "Supervisor unexpectedly accepted an invalid backup encryption password"
    )
    message = str(result.get("message") or result.get("error") or "").lower()
    assert any(word in message for word in ("password", "decrypt", "invalid key")), (
        "Supervisor did not explicitly report an encryption/password failure"
    )
    status = shell.run_check("cat " + status_file, timeout=30)
    # Genuine HAOS 18.3 observation: the CLI may exit 0 despite returning
    # Supervisor {"result":"error","message":"Invalid password ..."}.
    # The explicit JSON response plus immutable backup/share bytes decides
    # the outcome; process exit status alone is NOT authoritative.
    assert len(status) == 1 and status[0].strip().isdigit(), (
        "HA CLI did not produce a valid process exit status"
    )
    shell.run_check("rm -f " + result_file + " " + status_file, timeout=30)

    after = shell.run_check("sha256sum " + marker, timeout=45)
    assert len(after) == 1 and after[0].split()[0] == before[0].split()[0], (
        "Incorrect password unexpectedly modified actual HAOS share bytes"
    )
    assert any(line.strip() == mutated for line in shell.run_check("cat " + marker)), (
        "Synthetic share marker was restored/removed despite invalid password"
    )
    assert shell.run_check("sha256sum " + archive, timeout=45)[0].split()[0] == (
        archive_before[0].split()[0]
    ), "Encrypted full backup archive was mutated by denied restore"
    supervisor = shell_json("ha supervisor info --no-progress --raw-json", timeout=45)
    core = shell_json("ha core info --no-progress --raw-json", timeout=45)
    assert supervisor.get("result") == "ok" and core.get("result") == "ok"
    assert (core.get("data") or {}).get("version") == expected_core, (
        "Incorrect password altered Core or made it unavailable"
    )
    print("P8JK_REAL_HAOS_WRONG_PASSWORD_REJECTED_NO_DATA_MUTATION_PASS")


@pytest.mark.dependency(depends=["test_hars_real_supervisor_store"])
@pytest.mark.timeout(900)
def test_hars_real_official_file_editor_app_lifecycle_qemu(shell, shell_json):
    """Install/start/stop/uninstall real official File editor on disposable HAOS.

    Unlike earlier local Rust fixtures this is the genuine Supervisor Store
    core_configurator App. The nine original user Apps are NOT all validated.
    """
    from time import sleep as _sleep
    import os as _os
    import sys as _sys
    from pathlib import Path as _Path
    _sys.path.insert(0,str(_Path(_os.environ["GITHUB_WORKSPACE"])/"scripts"))
    from p8jl_app_state_recovery import execute_and_observe as _observed
    from p8jl_supervisor_watchdog import prepare_manual_stop as _manual_stop

    slug = "core_configurator"  # Public, fixed, official Store identifier.
    initial = shell_json(
        "ha apps info " + slug + " --no-progress --raw-json", timeout=90
    )
    assert initial.get("result") == "ok", (
        "Genuine official File Editor not discoverable in Supervisor Store"
    )
    available = initial.get("data") or {}
    assert available.get("slug") == slug, (
        "Supervisor returned metadata for the wrong App"
    )
    latest = available.get("version_latest")
    assert isinstance(latest, str) and latest.strip(), (
        "Official File Editor lacks a concrete Store version"
    )
    assert available.get("state") == "unknown", (
        "Disposable HAOS guest already has File Editor installed"
    )
    assert available.get("repository") in ("core", None), (
        "File Editor lookup resolved to a nonofficial repository"
    )

    installed_by_test = False
    try:
        result = shell_json(
            "ha apps install " + slug + " --no-progress --raw-json", timeout=420
        )
        assert result.get("result") == "ok", (
            "Official Supervisor File Editor installation failed"
        )
        installed_by_test = True
        _manual_stop(shell, shell_json, slug)
        installed_info = shell_json(
            "ha apps info " + slug + " --no-progress --raw-json", timeout=90
        )
        assert installed_info.get("result") == "ok"
        data = installed_info.get("data") or {}
        assert data.get("slug") == slug, (
            "Supervisor could not attest real installed File Editor App"
        )
        assert isinstance(data.get("version"), str) and data["version"], (
            "Real File Editor installed version unavailable"
        )
        assert data.get("state") in ("stopped", "started"), (
            "Unexpected installed File Editor container state"
        )
        if data.get("state") != "started":
            _observed(shell_json, "start", slug, sleeper=_sleep)

        for _ in range(30):
            observed = shell_json(
                "ha apps info " + slug + " --no-progress --raw-json", timeout=90
            )
            assert observed.get("result") == "ok"
            state = (observed.get("data") or {}).get("state")
            if state == "started":
                break
            _sleep(2)
        else:
            raise AssertionError("Real File Editor never reached started state")

        # Check availability of genuine App logs without printing their content
        # (which may include credentials or other untrusted content).
        shell.run_check(
            "ha apps logs " + slug + " --no-progress >/dev/null", timeout=45
        )
        stopped_info = _observed(
            shell_json, "stop", slug, shell=shell, sleeper=_sleep
        )
        clean_stop = stopped_info.get("state") == "stopped"
        proven_upstream_error_stop = (
            stopped_info.get("state") == "error"
            and stopped_info.get("_process_stopped_verified") is True
            and stopped_info.get("_docker_container_removed_verified") is True
            and stopped_info.get("_supervisor_error_state_observed") is True
        )
        assert clean_stop or proven_upstream_error_stop, (
            "Official File Editor still running or unverifiable after stop"
        )
        if proven_upstream_error_stop:
            # Retain Supervisor ERROR rather than falsifying STOPPED.
            # Docker container existed before POST and disappeared afterwards.
            # Actual exit code is not available once Supervisor removes it.
            print("P8JL_SUPERVISOR_ERROR_BUT_GUEST_DOCKER_REMOVAL_PROVEN")
        print("P8JL_REAL_OFFICIAL_CORE_CONFIGURATOR_START_STOP_PASS")
    finally:
        if installed_by_test:
            final = _observed(shell_json, "uninstall", slug, sleeper=_sleep)
            assert final.get("state") == "unknown", (
                "Real File Editor persists in disposable guest after uninstall"
            )
    print("P8JL_REAL_OFFICIAL_FILE_EDITOR_INSTALL_START_STOP_UNINSTALL_PASS")


@pytest.mark.dependency(depends=["test_hars_real_official_file_editor_app_lifecycle_qemu"])
@pytest.mark.timeout(1100)
def test_hars_real_official_file_editor_partial_restore_bytes_qemu(shell, shell_json):
    """Real native Supervisor partial backup/restore of actual File Editor app data.

    The bytes are synthetic and written ONLY in a GitHub-hosted disposable KVM
    guest, inside the canonical Supervisor-owned directory of the official App.
    """
    import re as _re
    from time import sleep as _sleep

    slug = "core_configurator"
    # Home Assistant Supervisor renamed App data from addons/data to apps/data.
    # Select a real existing directory and reject stale/ambiguous duplicates:
    # no invented mount, data-file fallback or test-only mkdir.
    app_roots = (
        "/mnt/data/supervisor/apps/data/",
        "/mnt/data/supervisor/addons/data/",
    )
    candidate_dirs = [root + slug for root in app_roots]
    data_dir = None
    marker = None
    original = "P8JM_REAL_FILE_EDITOR_ORIGINAL_20261022"
    mutated = "P8JM_REAL_FILE_EDITOR_MUTATED_20261022"
    installed_by_test = False
    backup_slug = None
    try:
        before = shell_json(
            "ha apps info " + slug + " --no-progress --raw-json", timeout=90
        )
        assert before.get("result") == "ok"
        assert (before.get("data") or {}).get("state") == "unknown", (
            "QEMU File Editor unexpectedly already installed before this case"
        )
        result = shell_json(
            "ha apps install " + slug + " --no-progress --raw-json", timeout=420
        )
        assert result.get("result") == "ok", "Official real File Editor install denied"
        installed_by_test = True
        app = shell_json(
            "ha apps info " + slug + " --no-progress --raw-json", timeout=90
        ).get("data") or {}
        assert app.get("slug") == slug and app.get("state") in (
            "started", "stopped"
        ), "Real File Editor not installed"
        original_version = app.get("version")
        assert isinstance(original_version, str) and original_version
        available_roots = shell.run_check(
            "sh -c 'for path in " + " ".join(candidate_dirs)
            + "; do if test -d \"$path\"; then printf \"%s\\n\" \"$path\"; fi; done'",
            timeout=40,
        )
        assert len(available_roots) == 1 and available_roots[0].strip() in (
            candidate_dirs
        ), "Real Supervisor File Editor App data root is missing or ambiguous"
        data_dir = available_roots[0].strip()
        marker = data_dir + "/rusty_home_file_editor_roundtrip_marker.txt"
        shell.run_check("sh -c 'echo " + original + " > " + marker + "'")
        assert any(line.strip() == original for line in shell.run_check(
            "cat " + marker, timeout=30
        )), "Cannot write synthetic marker in actual Supervisor App data"
        original_hash = shell.run_check("sha256sum " + marker, timeout=45)
        assert len(original_hash) == 1 and _re.fullmatch(
            r"[0-9a-f]{64}", original_hash[0].split()[0]
        )

        saved = shell_json(
            'ha backups new --name "Rusty Home Real File Editor App Data" '
            '--app ' + slug + " --no-progress --raw-json", timeout=420
        )
        assert saved.get("result") == "ok", (
            "Actual Supervisor selected-app partial backup failed"
        )
        backup_slug = (saved.get("data") or {}).get("slug")
        assert isinstance(backup_slug, str) and _re.fullmatch(
            r"[0-9a-f]{8,16}", backup_slug
        ), "Invalid partial backup returned by Supervisor"
        reply = shell_json(
            "ha backups info " + backup_slug + " --no-progress --raw-json",
            timeout=90,
        )
        assert reply.get("result") == "ok"
        meta = reply.get("data") or {}
        assert meta.get("slug") == backup_slug and meta.get("type") == "partial"
        apps = meta.get("addons")
        assert isinstance(apps, list), "Partial backup has no declared app list"
        included = []
        for item in apps:
            if isinstance(item, str):
                included.append(item)
            elif isinstance(item, dict):
                included.append(item.get("slug"))
        assert slug in included, (
            "Real Supervisor partial backup omitted the target File Editor App"
        )
        archive = "/mnt/data/supervisor/backup/" + backup_slug + ".tar"
        before_archive = shell.run_check("sha256sum " + archive, timeout=60)
        assert len(before_archive) == 1 and _re.fullmatch(
            r"[0-9a-f]{64}", before_archive[0].split()[0]
        ), "Real partial File Editor backup archive is missing"

        shell.run_check("sh -c 'echo " + mutated + " > " + marker + "'")
        assert any(line.strip() == mutated for line in shell.run_check(
            "cat " + marker, timeout=30
        )), "Synthetic File Editor marker was not changed after backup"

        result = shell_json(
            "ha backups restore " + backup_slug + " --app " + slug
            + " --no-progress --raw-json",
            timeout=650,
        )
        assert result.get("result") == "ok", (
            "Official Supervisor refused real File Editor partial data restore"
        )
        restored = False
        for _ in range(50):
            try:
                recovered_hash = shell.run_check(
                    "sha256sum " + marker, timeout=30
                )
                app_after = shell_json(
                    "ha apps info " + slug + " --no-progress --raw-json",
                    timeout=45,
                )
                supervisor = shell_json(
                    "ha supervisor info --no-progress --raw-json", timeout=45
                )
                core = shell_json(
                    "ha core info --no-progress --raw-json", timeout=45
                )
                state = app_after.get("data") or {}
                if (
                    len(recovered_hash) == 1
                    and recovered_hash[0].split()[0] == original_hash[0].split()[0]
                    and app_after.get("result") == "ok"
                    and state.get("slug") == slug
                    and state.get("version") == original_version
                    and state.get("state") in ("started", "stopped")
                    and supervisor.get("result") == "ok"
                    and core.get("result") == "ok"
                ):
                    restored = True
                    break
            except Exception:
                pass
            _sleep(5)
        assert restored, (
            "Actual File Editor App bytes, version and Core health not recovered"
        )
        assert any(line.strip() == original for line in shell.run_check(
            "cat " + marker, timeout=30
        )), "File Editor restored hash matched but synthetic marker missing"
        assert shell.run_check("sha256sum " + archive, timeout=45)[0].split()[0] == (
            before_archive[0].split()[0]
        ), "Real partial backup archive altered by restoration"
        print("P8JM_REAL_OFFICIAL_FILE_EDITOR_DATA_BYTES_RESTORED_PASS")
    finally:
        if installed_by_test:
            uninstalled = shell_json(
                "ha apps uninstall " + slug + " --no-progress --raw-json",
                timeout=240,
            )
            assert uninstalled.get("result") == "ok", (
                "Test-installed File Editor cleanup failed after real restore"
            )
            post = shell_json(
                "ha apps info " + slug + " --no-progress --raw-json", timeout=90
            )
            assert post.get("result") == "ok" and (
                post.get("data") or {}
            ).get("state") == "unknown", "Real File Editor persisted after cleanup"
        if backup_slug and _re.fullmatch(r"[0-9a-f]{8,16}", backup_slug):
            removed = shell_json(
                "ha backups remove " + backup_slug
                + " --no-progress --raw-json", timeout=120
            )
            assert removed.get("result") == "ok", (
                "Synthetic File Editor partial backup cleanup failed"
            )
