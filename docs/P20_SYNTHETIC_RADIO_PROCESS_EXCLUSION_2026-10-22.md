# P20 — Local process exclusion for a synthetic radio (22 Oct 2026)

## Real OS mechanism, no real radio access

P20 extends the earlier synthetic fencing-epoch tests with a genuine
nonblocking Linux flock between separate executable processes.
The only available CLI actions are --lab-radio-lease PRIVATE_DIR hold|probe.
Neither command accepts a USB identifier, Proxmox device or radio name.

The private fixture must be an owner-only, real (not symlink) immediate
child of this checkout's runtime/ directory. The synthetic lockfile is
opened using O_NOFOLLOW, O_CLOEXEC with mode 0600 and checked via fstat
for owner, regular type, mode, link count and zero length. Its inode
is re-checked after acquiring an exclusive nonblocking FD lock.
Root, group-accessible folders/files and symlinks fail closed.

## Evidence

The real Python child-process test:
- admits one holder but refuses another owner immediately;
- demonstrates a genuine SIGKILL releases the OS advisory lock;
- allows a different, private fixture root independently;
- rejects bad directory/file permissions, symlinked lockfiles and alias
  directories; ensures the dummy link target was never changed.
Native Rust unit tests check RAII exclusivity and private directory rules.
All artifacts and child processes are temporary and cleaned up.

The independent GitHub rust-lab workflow must successfully run the exact
PR SHA before merge. Local tests are necessary but not CI certification.

## Critical limitation

Advisory flock only coordinates cooperating local processes using the
same filesystem and lock. It cannot stop an HAOS VM, another host,
uncooperative root user, USB passthrough owner, Zigbee/Thread/ZWAVE
radio or Matter fabric from issuing real commands. It provides no real
hardware ownership or production cutover evidence. All outputs say
real_radio_access=false and production_ready=false. Thus the 15/15
release gates remain blocked, including single_radio_owner.
No HAOS synthetic VM, user credentials, USB, radios, services or devices touched.
