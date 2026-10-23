# Local App container built only from an explicitly compiled static musl Rust binary.
# The build context must provide "rusty-home" (scripts/build... creates it).
FROM scratch
LABEL io.hass.version="0.1.0-alpha.1" \
      io.hass.type="app" \
      io.hass.arch="amd64"
COPY --chmod=0555 rusty-home /usr/local/bin/rusty-home
# Preserve the first-party MIT copyright and permission notice in distributable images.
COPY --chmod=0444 LICENSE /usr/share/licenses/rusty-home/LICENSE
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/rusty-home", "--supervisor-live", "--host", "home-assistant", "--ingress", "--serve", "0.0.0.0:8099"]
