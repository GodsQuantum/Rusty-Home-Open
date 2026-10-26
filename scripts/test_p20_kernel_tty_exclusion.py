#!/usr/bin/env python3
"""Native Linux TIOCEXCL kernel owner admission test on a NEW ephemeral PTY.
Real open(2)/ioctl(2) cross-process behavior; NEVER open any actual USB/radio.
"""
import errno
import fcntl
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import termios

assert os.geteuid() != 0, "Unprivileged isolated CI/DEV process only"
assert os.uname().sysname == "Linux"
assert os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"
assert hasattr(termios, "TIOCEXCL") and hasattr(termios, "TIOCNXCL")
ROOT = Path(__file__).resolve().parent.parent
assert (ROOT / "Cargo.toml").is_file()
master, slave = os.openpty()
path = os.ttyname(slave)
assert path.startswith("/dev/pts/") and path[9:].isdigit(), path
flags = os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK | os.O_CLOEXEC | os.O_NOFOLLOW
holder = None
try:
    child = (
        "import fcntl,os,signal,termios,sys;"
        " fd=int(sys.argv[1]);"
        " fcntl.ioctl(fd,termios.TIOCEXCL);"
        " print('PTY_EXCLUSIVE_READY',flush=True);"
        " signal.pause()"
    )
    holder = subprocess.Popen(
        [sys.executable, "-c", child, str(slave)], pass_fds=(slave,),
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
        env={"PATH": "/usr/bin:/bin"},
    )
    readable, _, _ = select.select([holder.stdout], [], [], 5)
    assert readable and holder.stdout.readline().strip() == "PTY_EXCLUSIVE_READY"
    assert holder.poll() is None, "exclusive kernel owner exited"
    # Keep one trusted pre-open recovery descriptor: TIOCEXCL is a TTY flag,
    # NOT a flock. It need not clear when just the controller process dies.
    def require_busy():
        try:
            opened = os.open(path, flags)
        except OSError as exc:
            assert exc.errno == errno.EBUSY, ("expected kernel TIOCEXCL EBUSY", exc.errno)
        else:
            os.close(opened)
            raise AssertionError("second process admitted while kernel exclusive owner lives")

    require_busy()
    holder.send_signal(signal.SIGSTOP)
    require_busy()  # a suspended owner still holds its kernel file descriptor
    holder.send_signal(signal.SIGKILL)
    holder.wait(timeout=5)
    assert holder.returncode == -signal.SIGKILL
    holder.stdout.close()
    holder.stderr.close()
    holder = None

    # A surviving server-side reference preserves TIOCEXCL even after the
    # controller was killed. Never claim that SIGKILL alone restores access.
    require_busy()
    fcntl.ioctl(slave, termios.TIOCNXCL)  # explicit trusted recovery action
    recovered = os.open(path, flags)
    try:
        fcntl.ioctl(recovered, termios.TIOCEXCL)
        # A recovered owner must fence a second candidate exactly the same.
        require_busy()
        fcntl.ioctl(recovered, termios.TIOCNXCL)
        parallel = os.open(path, flags)  # kernel permits it only after explicit release
        os.close(parallel)
    finally:
        os.close(recovered)

    with tempfile.TemporaryDirectory(prefix="p20-pty-symlink-") as tmp:
        link = Path(tmp) / "untrusted-device-alias"
        link.symlink_to(path)
        try:
            bad = os.open(link, flags)
        except OSError as exc:
            assert exc.errno == errno.ELOOP, ("nofollow symlink rejection", exc.errno)
        else:
            os.close(bad)
            raise AssertionError("O_NOFOLLOW opened untrusted TTY alias")
    print("P20_REAL_TIOCEXCL_SIGSTOP_DENIED_SIGKILL_STILL_LOCKED_EXPLICIT_RELEASE_PASS")
    print("P20_NO_USB_NO_RADIO_NO_VM100_NO_PRODUCTION_AUTHORIZATION_PASS")
finally:
    if holder is not None:
        holder.kill()
        holder.wait(timeout=5)
        holder.stdout.close()
        holder.stderr.close()
    if slave >= 0:
        os.close(slave)
    os.close(master)
