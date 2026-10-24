# Deliberately inert, nonprivileged local App installed only in isolated HAOS.
FROM scratch
LABEL io.hass.version="1.0.0" io.hass.type="app" io.hass.arch="amd64"
COPY --chmod=0555 guest_supervisor_live /fixture
ENV HARS_PROBE_STAY_RUNNING=1
USER 1000:1000
ENTRYPOINT ["/fixture"]
