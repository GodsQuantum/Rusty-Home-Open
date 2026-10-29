# Rusty Home — project status / État du projet

**Status on 29 October 2026: DEVELOPMENT PAUSED / DÉVELOPPEMENT ARRÊTÉ.**

> [!CAUTION]
> **NOT PRODUCTION READY — 0/15 release gates certified.** Do not install on a real household, rely on this software for security or backups, transfer radio ownership, or import a private HAOS configuration.

The original maintainer stopped active work because of the time and resource requirements. The existing MIT-licensed source and Git history remain public for people who might like to continue. The repository is intentionally not GitHub-archived so issues, forks and pull requests stay possible. **No support, review, merge or release schedule is promised.** There are no supported production installation instructions or certified binaries.

## What is available

Experimental Rust state/event/automation components, a read-only synthetic dashboard, genuine isolated Home Assistant Core/Supervisor integration laboratories, backup and security fixtures, and CI. These are research and laboratory results, NOT a finished home automation application.

## Known gaps and release conditions

The machine release gate currently reports **0/15 certified (9 lab-partial, 6 blocked)**. The remaining work includes true authenticated host and user writes, HAOS backup transformation and rollback, 9 real Apps and 29 representative Core/HACS integration lifecycles, physical radio ownership, non-synthetic drivers, WebSocket resilience, 72-hour full-stack endurance, CPU/RAM measurements, reproducible packaging, licenses, fuzz/security tests and independent sign-off.

References: [certification master plan](PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md), [evidence register](CERTIFICATION_EVIDENCE_REGISTER.md), [contributing](../CONTRIBUTING.md). Older forecasts were planning assumptions, not delivery commitments.

## Open contributions (NOT merged into main)

- [PR #264](https://github.com/GodsQuantum/Rusty-Home-Open/pull/264): RBAC; one HAOS test was failing on that branch at the checkpoint.
- [PR #266](https://github.com/GodsQuantum/Rusty-Home-Open/pull/266): synthetic backup secret protection.
- [PR #268](https://github.com/GodsQuantum/Rusty-Home-Open/pull/268): disposable Supervisor QEMU update preflight and diagnostics.
- [PR #269](https://github.com/GodsQuantum/Rusty-Home-Open/pull/269): synthetic Rust benchmark budgets, NOT full-stack performance certification.

Volunteers should inspect and independently validate these branches instead of assuming that green CI makes them safe to merge or deploy.

## Contributions and maintenance

You are welcome to fork the repository, open an issue with a reproducible sanitized test, or offer a focused PR. The maintainer has paused development and may not respond. Do not submit household data, real credentials, production photos or other identifying material. Use isolated QEMU and synthetic fixtures. There is no product release or deployment plan.

**Cost controls:** Scheduled weekly GitHub Actions were removed at shutdown. Only normal push/PR/manual CI triggers remain for any future external contributions. Rusty Home Paperclip automation and local builds should be stopped.

## Français

**Projet suspendu pour raisons de temps et de ressources.** Code conservé en public pour permettre à d'autres de poursuivre, sans engagement de maintenance. Rusty Home **n'est pas un logiciel utilisable et ne doit pas être installé chez soi** ; aucun des quinze critères de production n'est certifié. Les forks et pull requests restent possibles.
