#!/usr/bin/env python3
"""Disable only real File Editor watchdog in disposable HAOS via official API.

Uses the official CLI container's already-authorized Supervisor token in-place;
never reads, prints or exports credentials to GitHub runner.
"""
APP = "core_configurator"

def prepare_manual_stop(shell, shell_json, slug):
    if slug != APP:
        raise ValueError("only disposable official File Editor may be configured")
    # curl executes INSIDE authenticated Supervisor CLI container; token never
    # crosses a process/host boundary nor appears in stdout/our command string.
    command = (
        "docker exec hassio_cli sh -c '"
        "test -n \"$SUPERVISOR_TOKEN\" && "
        "curl --fail --silent --show-error --max-time 30 "
        "-H \"Authorization: Bearer $SUPERVISOR_TOKEN\" "
        "-H \"Content-Type: application/json\" "
        "--data-binary \"{\\\"watchdog\\\":false,\\\"boot\\\":\\\"manual\\\"}\" "
        "http://supervisor/addons/core_configurator/options -o /dev/null'"
    )
    shell.run_check(command, timeout=45)
    status = shell_json("ha apps info " + APP + " --no-progress --raw-json", timeout=75)
    if not isinstance(status, dict) or status.get("result") != "ok":
        raise RuntimeError("Could not verify Supervisor App options after write")
    data = status.get("data")
    if not isinstance(data, dict) or data.get("slug") != APP:
        raise RuntimeError("Wrong App metadata after Supervisor options write")
    if data.get("watchdog") is not False or data.get("boot") != "manual":
        raise RuntimeError("Supervisor did not confirm File Editor manual boot and watchdog off")
    return {"watchdog":False,"boot":"manual"}
