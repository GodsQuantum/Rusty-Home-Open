#!/usr/bin/env python3
"""Create an ignored isolated comparison workspace using real pinned sources.

The generated code runs ONLY synthetic identifiers, never production .storage,
USB/network drivers, MQTT broker or device discovery.
"""
from pathlib import Path
import shutil
ROOT=Path(__file__).resolve().parent.parent
STAGE=ROOT/"runtime/bench-equal"
(STAGE/"src").mkdir(parents=True,exist_ok=True)
(STAGE/"ruview/crates").mkdir(parents=True,exist_ok=True)
for name in ("homecore",):
    source=ROOT/"references/RuView-HOMECORE/v2/crates"/name
    dest=STAGE/"ruview/crates"/name
    if not dest.exists():
        shutil.copytree(source,dest)
(STAGE/"ruview/Cargo.toml").write_text('[workspace]\nresolver = "2"\nmembers = ["crates/homecore"]\n')
(STAGE/"Cargo.toml").write_text(f'''[workspace]

[package]
name = "ha-bench-equal"
version = "0.0.1"
edition = "2024"
publish = false

[dependencies]
homecore = {{path = "ruview/crates/homecore"}}
hc-state = {{path = "../../references/homeCore-io/crates/hc-state"}}
hc-types = {{path = "../../references/homeCore-io/crates/hc-types"}}
serde_json = "1"
tokio = {{version = "1",features = ["rt-multi-thread", "macros"]}}
tempfile = "3"
''')
print("Synth-only upstream comparison workspace:",STAGE)
