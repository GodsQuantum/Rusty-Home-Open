FROM scratch
LABEL io.hass.version="0.1.0" \
      io.hass.type="app" \
      io.hass.arch="amd64"
COPY --chmod=0555 guest_core_users_probe /hars-core-users
ENV HARS_P8H_QEMU_ONLY=1
ENV HARS_CORE_PROBE_STAY_RUNNING=1
USER 1000:1000
ENTRYPOINT ["/hars-core-users"]
