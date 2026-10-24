# Durable dispatch fence

Experimental, opt-in only. A two-phase redb commit locks one App before a Supervisor preview POST. On failure or restart the operation stays uncertain and cannot be retried automatically. Successful submission is not verified completion. All 15 production gates remain uncertified.

## Typed origin-to-watch registration

The durable fence now carries the actor subject directly from the already audited permit, rather than accepting a separate caller-supplied label. After a successful authenticated lab submission returns an official job ID, an install watch may be reconstructed idempotently from the accepted dispatch record after process restart. Wrong vendor, action, missing receipt or forged job cannot produce a watch. Updates are deliberately excluded until a trustworthy pre-update version is captured. This is an experimental observation binding, not an end-user authorization grant or production certification.
