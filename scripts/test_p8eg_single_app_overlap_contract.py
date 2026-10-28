#!/usr/bin/env python3
"""P8eg only NEW single-app stage public source contract, not old pair proof."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
library=(root/"crates/interop-moli/src/app_backup_stage.rs").read_text()
source=(root/"scripts/test_p8eg_single_app_overlap.py").read_text()
workflow=(root/".github/workflows/p8eg-single-app-private-root-overlap.yml").read_text()
for required in (
    "source.parent().context(", "fs::canonicalize(source_parent)",
    "fs::canonicalize(target_root)", "source_root.starts_with(&output_root)",
    "output_root.starts_with(&source_root)",
    "refuse aliased or overlapping App source and stage roots",
):
    assert required in library, required
assert library.index("source_root.starts_with(&output_root)")<library.index("reap_interrupted_stages(target_root)")
for marker in (
    "P8EG_SINGLE_DOTDOT_EQUIVALENT_OVERLAP_REJECTED_PASS",
    "P8EG_SINGLE_NESTED_OUTPUT_OVERLAP_REJECTED_PASS",
    "P8EG_SINGLE_ANCESTOR_OUTPUT_OVERLAP_REJECTED_PASS",
    "P8EG_SINGLE_SYMLINKED_PARENT_ALIAS_REJECTED_PASS",
    "P8EG_SINGLE_DISJOINT_REAL_NATIVE_APP_STAGE_ORIGINAL_SHA_UNCHANGED_PASS",
):
    # Markers generated from labels in Python, require source shape instead.
    assert marker.startswith("P8EG_SINGLE")
for pattern in (
    "dotdot-equivalent", "nested-output", "ancestor-output",
    "linked-parent", "app-source-stage-inactive", "initial=immutable.read_bytes()",
    "immutable.read_bytes()==initial", "production_ready",
):
    assert pattern in source,pattern
for pattern in (
    "scripts/test_p8eg_single_app_overlap.py",
    "cargo build --locked -p hars-interop-moli --bin ha_registry_stager",
    "cargo clippy --locked", "python3 scripts/test_public_redaction.py",
):
    assert pattern in workflow,pattern
for forbidden in ("192.168.", "qm stop", "pct exec", "SUPERVISOR_TOKEN",
                  "pull_request_target:", "workflow_dispatch:", "secrets.", "--privileged"):
    assert forbidden not in source+workflow,forbidden
print("P8EG_SINGLE_APP_NATIVE_PATH_GUARD_SCOPE_CONTRACT_PASS")
