#!/usr/bin/env python3
"""P8di: real official App process SIGKILL/restart and independent container recreation.

Only on a disposable GitHub runner, with one verified synthetic HAOS App data
directory already staged by Rust. No ingress, host networking or real secrets.
"""
import hashlib
import os
import time

def verify_restart(run, name, launch, data_dir, marker_path, marker, archive, sha, image_id):
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_TEMP")
    assert os.geteuid() != 0
    assert "--network" in launch and launch[launch.index("--network")+1] == "none"
    assert "--read-only" in launch and "--cap-drop" in launch
    assert launch[-1] == image_id and image_id.startswith("sha256:")
    assert str(data_dir) in launch[launch.index("--mount")+1]
    note = "p8di_synthetic_volume_write.txt"
    payload = "P8DI_FAKE_NONSECRET_DURABLE_APP_VOLUME_DATA_V1"
    def digest():
        with archive.open("rb") as stream:
            return hashlib.file_digest(stream, "sha256").hexdigest()
    def running():
        result = run(["podman", "inspect", name, "--format", "{{.State.Running}}"], expect=False)
        return result.returncode == 0 and result.stdout.strip() == "true"
    def check_live():
        for _ in range(40):
            if running():
                probe = run(["podman", "exec", name, "sh", "-c",
                             "cat /data/rusty_home_p8cu_nonsecret_synthetic_marker.txt; "
                             "cat /data/" + note], expect=False, timeout=15)
                if probe.returncode == 0 and probe.stdout == marker.decode() + payload:
                    return
            time.sleep(.5)
        raise AssertionError("Official App did not remain running with both persistent bytes")
    assert marker_path.read_bytes() == marker and digest() == sha
    # A genuine process writes to the owned /data bind, never to the read-only image.
    write = run(["podman", "exec", name, "sh", "-c",
                 "umask 077; printf %s " + payload + " > /data/" + note], expect=False)
    assert write.returncode == 0, write.stderr[-500:]
    assert (data_dir/note).read_text() == payload
    assert (data_dir/note).stat().st_mode & 0o077 == 0
    print("P8DI_REAL_OFFICIAL_APP_PRIVATE_RW_VOLUME_WRITE_PASS", flush=True)
    run(["podman", "kill", "--signal", "KILL", name], timeout=35)
    for _ in range(40):
        if not running():
            break
        time.sleep(.25)
    assert not running(), "App unexpectedly auto-restarted after SIGKILL"
    assert run(["podman", "inspect", name, "--format",
                "{{.HostConfig.NetworkMode}}"]).stdout.strip() == "none"
    assert marker_path.read_bytes() == marker
    assert (data_dir/note).read_text() == payload
    print("P8DI_GENUINE_OFFICIAL_APP_SIGKILL_PROCESS_STOPPED_VOLUME_INTACT_PASS", flush=True)
    run(["podman", "start", name], timeout=60)
    check_live()
    print("P8DI_GENUINE_OFFICIAL_APP_RESTART_RESTORED_AND_NEW_BYTES_PERSIST_PASS", flush=True)
    # A second fresh container process with the SAME pinned image and same
    # owned bind tests durable reattachment, not only Podman's restart state.
    run(["podman", "rm", "--force", name], timeout=45)
    run(launch, timeout=75)
    check_live()
    assert digest() == sha and marker_path.read_bytes() == marker
    assert (data_dir/note).read_text() == payload
    assert run(["podman", "inspect", name, "--format",
                "{{json .HostConfig.PortBindings}}"]).stdout.strip() in ("{}", "null")
    print("P8DI_NEW_ROOTLESS_OFFLINE_APP_CONTAINER_SAME_PINNED_IMAGE_DATA_SURVIVES_PASS", flush=True)
    print("P8DI_ORIGINAL_GENUINE_HAOS_APP_BACKUP_UNMODIFIED_NO_INGRESS_CLAIM_PASS", flush=True)
