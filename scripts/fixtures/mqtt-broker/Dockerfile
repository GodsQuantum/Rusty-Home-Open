FROM debian:13-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends mosquitto ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY mosquitto.conf /etc/mosquitto/mosquitto.conf
USER 1000:1000
ENTRYPOINT ["mosquitto","-c","/etc/mosquitto/mosquitto.conf"]
