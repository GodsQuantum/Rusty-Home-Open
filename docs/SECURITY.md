# Security boundaries

There are no production home credentials, actual backup, radio fabric
keys or real inventory in this public source snapshot.

Never deploy it on an active home, mount a real radio, or use household
credentials. Lab HTTP listens only on loopback and denies mutating routes.
The lab Unix flock does not fence other hosts or a Home Assistant VM.
