#!/usr/bin/env python3
"""Adversarial check: a counter or made-up completion cannot move progress."""
import copy,json
from check_development_progress import ROOT,validate
data=json.loads((ROOT/"config/development-progress.json").read_text())
release=json.loads((ROOT/"config/release-gates.json").read_text())
lab=json.loads((ROOT/"config/lab-evidence.json").read_text())
before=validate(data,release,lab)
assert before["technical_maturity_index"]["passed_levels"]==25
assert before["active_app_migration_workstream"]["verified"]==4
assert before["gate_partial_coverage"]["covered"]==9
assert before["production_certification"]["certified"]==0
assert before["laboratory_evidence"]["claims"]>=47
def bad(update):
    altered=copy.deepcopy(data)
    update(altered)
    try:validate(altered,release,lab)
    except (AssertionError,KeyError,TypeError,ValueError):
        return
    raise AssertionError("fabricated progress was accepted")
bad(lambda d:d["maturity_gates"][0].update(level=4))
bad(lambda d:d["maturity_gates"][0].update(evidence_id="fake_unverified_run"))
bad(lambda d:d["active_workstream"]["milestones"][4].update(verified=True))
bad(lambda d:d["active_workstream"]["milestones"][0].update(source_marker="FAKE_CI_SUCCESS"))
bad(lambda d:d["active_workstream"]["milestones"][2].update(evidence_id="haos_app_only_native_archive_asan_fuzz"))
bad(lambda d:d["active_workstream"]["milestones"][0].update(verified=False))
print("PASS: release gates, 4/10 App steps, 25/60 maturity, fabricated evidence or promotions denied")
