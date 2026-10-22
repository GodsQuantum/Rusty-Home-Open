#!/usr/bin/env python3
"""Bounded read-after-timeout for one disposable, real official Supervisor App.

A POST /addons/<slug>/start|stop|uninstall can complete after the CLI's
HTTP response times out. NEVER repeat an ambiguous POST. Only accept
a fresh GET /addons/<slug>/info with the exact observed target state.
No production host URL, user input or arbitrary App slugs.
"""
from time import sleep as _sleep

APP = "core_configurator"
ACTIONS = {"start": "started", "stop": "stopped", "uninstall": "unknown"}
# Both historical addon_ and current app_ Docker names are valid Supervisor names.
DOCKER_NAMES = frozenset(("addon_core_configurator", "app_core_configurator"))

def _timeout(error):
    message = str(error).lower()
    return any(value in message for value in (
        "context deadline exceeded",
        "timed out",
        "timeout",
        "awaiting headers",
    ))

def execute_and_observe(shell_json, action, slug, *,
                        shell=None, sleeper=_sleep, attempts=55, interval=3):
    if slug != APP or action not in ACTIONS:
        raise ValueError("only disposable genuine official File Editor lifecycle")
    if not isinstance(attempts, int) or isinstance(attempts,bool) or not 1<=attempts<=80:
        raise ValueError("invalid bounded Supervisor observation budget")
    if not isinstance(interval,(int,float)) or isinstance(interval,bool) or not 0<=interval<=5:
        raise ValueError("invalid bounded Supervisor interval")
    expected = ACTIONS[action]
    # Identify and attest the exact running official container BEFORE POST.
    # Supervisor may use modern app_ or legacy addon_ names. Its cleanup can
    # remove the container after SIGTERM; do not infer an unobserved exit code.
    pre_stop_running=False
    if action == "stop" and shell is not None:
        running=shell.run_check("docker ps --format '{{.Names}}'",timeout=45)
        matches=[name.strip() for name in running if name.strip() in DOCKER_NAMES]
        if len(matches)!=1:
            raise RuntimeError("No unique running official File Editor Docker container")
        container=matches[0]
        before=shell.run_check(
            "docker inspect --format '{{.State.Running}}|{{.Name}}' "+container,
            timeout=45,
        )
        pre_stop_running=(
            len(before)==1 and before[0].strip()=="true|/"+container
        )
        if not pre_stop_running:
            raise RuntimeError("Official File Editor Docker running proof failed")
    command = "ha apps " + action + " " + APP + " --no-progress --raw-json"
    try:
        write = shell_json(command, timeout=185)
    except Exception as error:
        if not _timeout(error):
            raise
        # An ambiguous timeout does not mean the POST was rejected: re-read
        # actual state, NEVER send a second POST.
    else:
        if not isinstance(write, dict) or write.get("result") != "ok":
            raise RuntimeError("Supervisor rejected File Editor operation")

    for poll in range(attempts):
        observed = None
        try:
            observed = shell_json(
                "ha apps info " + APP + " --no-progress --raw-json",
                timeout=45,
            )
        except Exception as error:
            if not _timeout(error):
                raise
        else:
            if (isinstance(observed,dict)
                and observed.get("result") == "ok"
                and isinstance(observed.get("data"),dict)
                and observed["data"].get("slug") == APP
                and observed["data"].get("state") == expected):
                return observed["data"]
        # See home-assistant/supervisor#6840: an intentional stop may be
        # reported as ERROR if the real app exits via SIGTERM. Supervisor
        # can remove the Docker container during its cleanup, so an inspect
        # of a no-longer-existing container is NOT evidence of ExitCode=143.
        # Prove the container ran BEFORE stop and was removed AFTER stop.
        # Two real Supervisor GETs and two complete Docker enumerations,
        # with watchdog off and manual boot, must agree. State remains ERROR.
        if (action == "stop" and shell is not None and pre_stop_running
            and isinstance(observed,dict)
            and observed.get("result") == "ok"
            and isinstance(observed.get("data"),dict)):
            info=observed["data"]
            if (info.get("slug") == APP and info.get("state") == "error"
                and info.get("watchdog") is False and info.get("boot") == "manual"):
                names_command="docker ps -a --format '{{.Names}}'"
                names=shell.run_check(names_command,timeout=45)
                if all(x.strip() not in DOCKER_NAMES for x in names):
                    sleeper(interval)
                    second=shell_json(
                        "ha apps info " + APP + " --no-progress --raw-json",timeout=45,
                    )
                    details=second.get("data") if isinstance(second,dict) else None
                    if (isinstance(second,dict) and second.get("result") == "ok"
                        and isinstance(details,dict)
                        and details.get("slug") == APP
                        and details.get("state") == "error"
                        and details.get("watchdog") is False
                        and details.get("boot") == "manual"):
                        after=shell.run_check(names_command,timeout=45)
                        if all(x.strip() not in DOCKER_NAMES for x in after):
                            return {
                                **details,
                                "_process_stopped_verified":True,
                                "_docker_container_removed_verified":True,
                                "_supervisor_error_state_observed":True,
                            }
        if poll + 1 < attempts:
            sleeper(interval)
    raise RuntimeError(
        "Genuine Supervisor File Editor never reached the requested state"
    )
