<p align="center"><img src="branding/logo.svg" width="820" alt="Rusty Home logo" /></p>
<h1 align="center">Rusty Home</h1>
<p align="center"><strong>Home automation, reimagined in Rust.</strong><br>A native, local-first home automation engine designed with the Home Assistant ecosystem in mind.</p>
<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.zh-CN.md">简体中文</a></p>
<p align="center">
<a href="https://github.com/GodsQuantum/Rusty-Home-Open/actions/workflows/ci.yml"><img src="https://github.com/GodsQuantum/Rusty-Home-Open/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
<img src="https://img.shields.io/badge/stage-early%20alpha-c7632c" alt="Early alpha"/>
<img src="https://img.shields.io/badge/runtime-Rust-bb6233" alt="Rust"/>
<a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-4b9b9d" alt="MIT license"/></a>
<img src="https://img.shields.io/badge/prod-NOT%20READY-bc5555" alt="Not production ready"/>
</p>

> [!WARNING]
> **Research prototype — not a Home Assistant replacement yet.** Do not connect Rusty Home to a production home, migrate a configuration or hand it radio ownership. All **15 mandatory production gates remain blocked**. The inventories of 29 custom integrations and nine HAOS Apps do NOT certify full runtime compatibility.

## A different foundation for home automation

Rusty Home is an independent Rust-first exploration of durable local state, safe automations, Home Assistant compatibility and reversible migration. It is **not a Home Assistant fork** or an installable distribution.

| Proven in isolated laboratories | Not yet certified |
| --- | --- |
| Native Rust state/event/automation prototypes, real SIGKILL recovery tests | Real household device drivers, radios and migration |
| A read-only dashboard for synthetic and signed app inventory | Production identity, authentication and host transport |
| Real isolated Home Assistant Core integration tests for selected HACS integrations | All 29 integrations and all nine Apps |
| Experimental HAOS image backup and restore paths | User data restore, actual rollback, single radio ownership |
| Strict release-gate checker and security-focused lab tests | Full-stack performance, packaging and third-party licensing |

## The laboratory interface

<p align="center"><img src="branding/lab-dashboard.png" width="760" alt="Rusty Home read-only dashboard with synthetic fixture data"/>
  <img src="branding/lab-dashboard-mobile.png" width="220" alt="Rusty Home read-only synthetic lab dashboard on mobile" /></p>
<p align="center"><sub>Actual Rusty Home web frontend captured via Playwright on a free GitHub-hosted Ubuntu runner, with four synthetic Apps and no household data.</sub></p>

## Architecture

~~~text
Rusty Home (experimental)
 ├── Native Rust state, events, automation, durability
 ├── Read-only dashboard and verified release gates
 ├── Optional isolated Home Assistant Python compatibility
 └── Future drivers, device ownership and Apps (not certified)
~~~

Rusty Home uses pinned upstream libraries where appropriate and does not redistribute reference checkouts. See [architecture](docs/ARCHITECTURE.md), [upstream inventory](docs/UPSTREAMS.md), [security](docs/SECURITY.md) and [production gates](docs/PRODUCTION_READINESS_2026-10-18.md).

## Quick start: development labs only

Requires Rust, Python 3 and a private, isolated Linux development workspace. Some advanced labs need Docker/QEMU.

~~~bash
git clone https://github.com/GodsQuantum/Rusty-Home-Open.git
cd Rusty-Home
cargo test --locked --all
python3 scripts/check_release_gate.py --expect-blocked
bash scripts/test-lab.sh
~~~

These tests do not control hardware or authorize live deployment. Never import a real HAOS backup or give the labs production credentials.

## Project links

[Status](STATUS.md) · [Roadmap](docs/ROADMAP.md) · [Verified evidence](docs/RELEASE_STATUS.md) · [Publication readiness](docs/PUBLIC_RELEASE_CHECKLIST.md) · [Contributing](CONTRIBUTING.md) · [Visual identity](branding/PALETTE.md)

**Legal:** Before public release, authorship, third-party licenses and historic sensitive data must be reviewed. See [provenance](docs/THIRD_PARTY_PROVENANCE.md). Rusty Home is not affiliated with or endorsed by Home Assistant / Nabu Casa.
