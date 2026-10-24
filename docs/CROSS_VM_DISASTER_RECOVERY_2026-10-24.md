# P8p — Independent fresh second-VM HAOS full restore (new, distinct scope)

**Status: ACTUAL QEMU SUCCESS 6/6, NOT_PRODUCTION_READY, 0/15 production gates.**

Already proven, DO NOT repeat: full restore within the same genuine HAOS 18.3 QEMU VM, encrypted full restore, wrong password refusal, real File Editor partial data recovery and App rollback. See [the canonical evidence index](CERTIFICATION_EVIDENCE_REGISTER.md). This experiment is materially different: backup transfer from a SOURCE VM to an independent, empty TARGET VM.

## Runbook and strict evidence

The dedicated workflow reuses the pinned official HAOS 18.3 QCOW2 image and upstream KVM test harness, in TWO separate QEMU launches. Each boot uses -snapshot, so the first VM's changes cannot become the second's base disk. The source records its OS boot ID, creates a recognizable synthetic share marker and a genuine Supervisor full backup, and uploads exactly one bounded archive to a loopback-only runner relay.

The relay streams and SHA-256-hashes only one 1-GiB-max synthetic tar file, fsyncs it, stores only slug, source boot ID, Core version, size and SHA-256, disallows overwrites and serves it only to the fresh guest over QEMU NAT. The target must prove that the source marker DOES NOT exist, the target boot ID differs, the transferred SHA-256 matches, the Supervisor recognizes the imported backup, then conduct actual full restore and verify the exact original share bytes and Core version. Two separate 3/3 JUnit checks reject missing or skipped cases.

This would prove **official HAOS to official HAOS disaster recovery** in two disposable guest VMs. It does NOT prove Rusty Home can replace Home Assistant Core itself, generic third-party databases, hardware radio ownership, private credentials, recovery across architectures, production OAuth or all 15 release gates. A live CI success is mandatory before recording this as verified. Do not confuse a successful same-guest round trip with separate-target recovery.

Workflow: .github/workflows/p8p-cross-vm-recovery.yml; source: scripts/p8p_cross_vm_backup_pytest.py; relay: scripts/p8p_cross_vm_backup_relay.py; strict JUnit: scripts/check_p8p_cross_vm_junit.py.

## État actuel et non-redondance

[PR #86](https://github.com/GodsQuantum/Rusty-Home-Open/pull/86) exécute cette preuve indépendante. La première révision a échoué à déclencher le workflow général à cause d'une erreur YAML (correctif sur SHA 39b778274be83510f6a4ed9757e648dbc67638dd), pas à cause d'une erreur de restauration HAOS. Attendre le véritable résultat de source + target avant de promouvoir cet élément dans le registre des preuves. L'ensemble du reste des restorations SAME-VM existe déjà et reste attesté.

## Verified CI result

Real pinned HAOS 18.3 source+fresh-target [run 40134632585](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40134632585) on SHA 161307bd227b279c73df5a33da759a19ed4ac1ae: source 3/3 PASS, target 3/3 PASS, both strict JUnit validators PASS. This is a bounded HAOS-to-HAOS recovery proof, NOT Rust native HA replacement certification.
