# Durable dispatch fence

Experimental, opt-in only. A two-phase redb commit locks one App before a Supervisor preview POST. On failure or restart the operation stays uncertain and cannot be retried automatically. Successful submission is not verified completion. All 15 production gates remain uncertified.
