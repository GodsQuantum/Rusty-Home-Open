# App-specific Docker image, built by genuine Supervisor only in ephemeral HAOS.
# This is a static musl test binary, never the real production service.
FROM scratch
LABEL io.hass.version="0.1.0" \
      io.hass.type="app" \
      io.hass.arch="amd64"
# HAOS guest curl writes the build-context binary without the executable bit.
COPY --chmod=0555 guest_supervisor_live /hars-supervisor-probe
# P8g is an explicitly compiled lab-only client targeting core_ssh only.
COPY --chmod=0555 guest_supervisor_lab_lifecycle /hars-lifecycle
# P8h validated dynamic operation preview is disabled in default Rust builds.
COPY --chmod=0555 guest_supervisor_managed_preview /hars-manager
# Private LAB-only writable audit directory for UID 1000 (nonroot).
# Supervisor /data may be root-owned: never chmod or chown host-mounted data.
COPY --chown=1000:1000 --chmod=0700 audit_layer/ /hars-lab-audit/
ENV HARS_P8H_QEMU_ONLY=1
ENV HARS_P8G_QEMU_ONLY=1
ENV HARS_PROBE_STAY_RUNNING=1
USER 1000:1000
ENTRYPOINT ["/hars-supervisor-probe"]
