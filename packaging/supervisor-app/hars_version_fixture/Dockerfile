# QEMU-only local versioned fixture. Both versions use the same static probe.
FROM scratch
LABEL io.hass.version="1.0.0" \
      io.hass.type="app" \
      io.hass.arch="amd64"
COPY --chmod=0555 guest_supervisor_live /version-fixture
ENV HARS_PROBE_STAY_RUNNING=1
USER 1000:1000
ENTRYPOINT ["/version-fixture"]
