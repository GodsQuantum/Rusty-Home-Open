# Rusty Home — experimental local Supervisor App

This folder is an **installable local development App package** for Home
Assistant OS on amd64. Its UI is a read-only inventory, **not a complete
Home Assistant replacement**. It has no lifecycle mutation UI and cannot
replace an existing Home Assistant instance.

The Supervisor App itself grants the official *manager* REST access necessary
to inspect Apps. Rusty Home's default binary exposes only GET requests, does
not include the feature-gated mutation transport, uses no devices, host
network, Docker socket or host-mounted folders, and runs as UID 1000 with
AppArmor/protected mode enabled.

### Build

Compile the control plane into a **static** musl binary for amd64:

    rustup target add x86_64-unknown-linux-musl
    cargo build --release --locked --target x86_64-unknown-linux-musl -p ha-rust-bridge

Copy the resulting binary to this directory as `rusty-home` before building
the image. The supplied build/test script can do this in a disposable
workspace; **never check in the binary**. The Dockerfile uses `scratch`.

The Supervisor configuration sets `ingress: true` and port 8099; it does
not publish a host port. Rusty Home trusts only requests from the official
ingress proxy IP `172.30.32.2` carrying a Supervisor-issued user identity.
Any other requests, including forged headers, receive HTTP 403. The server
reads the internal `SUPERVISOR_TOKEN` injected into the App container.

**Packaging warning:** this GitHub repository also includes disposable
laboratory App definitions. Do **not** add its entire root as a public
Supervisor App repository. This directory can be staged as a local
development App on a disposable HAOS test VM; no real production install,
migration or broad App publication has been performed.

Reference:
https://contributors.home-assistant.io/apps/reference/configuration/
https://contributors.home-assistant.io/apps/guides/ingress/
