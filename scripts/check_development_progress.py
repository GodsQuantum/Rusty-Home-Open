#!/usr/bin/env python3
"""Conservative auditable progress. NOT a product-completion forecast."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def validate(data,release,lab):
    assert data.get("schema_version")==1
    gates={g["id"]:g for g in release["required_gates"]}
    assert len(gates)==15
    scores=data.get("maturity_gates")
    assert isinstance(scores,list) and len(scores)==len(gates)
    assert {x["gate_id"] for x in scores}==set(gates)
    known={c["id"]:c for c in lab["claims"]}
    for g in scores:
        key=g["gate_id"]
        level=g.get("level")
        assert type(level) is int and 0<=level<=4,(key,level)
        proof=g.get("evidence_id")
        if proof is not None:
            assert proof in known,(key,proof)
        if level>=2:
            assert proof and known[proof]["release_gate"]==key,(key,proof)
            assert known[proof]["status"]=="verified_lab_only"
        if level>=3:
            # No full functionality level without an independently verified
            # E2E acceptance suite, not merely a single isolated CI job.
            document=g.get("full_integration_acceptance_file")
            assert document and isinstance(document,str)
            path=ROOT/document
            assert path.is_file() and path.is_relative_to(ROOT)
        if level==4:
            assert gates[key]["status"]=="certified"
            assert gates[key]["certification_artifact"] is not None
        assert isinstance(g.get("limitation"),str) and len(g["limitation"])>12
    lane=data["active_workstream"]
    items=lane["milestones"]
    assert len(items)==10 and len({m["id"] for m in items})==10
    seen_gap=False
    for item in items:
        assert isinstance(item.get("acceptance"),str) and len(item["acceptance"])>15
        done=item["verified"]
        assert type(done) is bool
        if done:
            assert not seen_gap,"milestones must complete in declared dependency order"
            id=item["evidence_id"]
            assert id in known and known[id]["status"]=="verified_lab_only"
            assert known[id]["release_gate"]=="supervisor_apps_restore"
            file=ROOT/item["source_file"]
            assert file.is_file() and file.is_relative_to(ROOT)
            assert item["source_marker"] in file.read_text(),item["id"]
        else:
            seen_gap=True
            assert item["evidence_id"] is None and item["source_file"] is None
            assert item["source_marker"] is None
    certified=sum(g["status"]=="certified" for g in gates.values())
    covered=sum(g["status"]=="lab_partial" for g in gates.values())
    maturity=sum(g["level"] for g in scores)
    active=sum(m["verified"] for m in items)
    return {
        "production_certification":{"certified":certified,"total":len(gates),"percent":round(100*certified/len(gates),1)},
        "gate_partial_coverage":{"covered":covered,"total":len(gates),"percent":round(100*covered/len(gates),1)},
        "technical_maturity_index":{"passed_levels":maturity,"total_levels":len(scores)*4,"percent":round(100*maturity/(len(scores)*4),1),"is_product_completion_percent":False},
        "active_app_migration_workstream":{"verified":active,"total":len(items),"percent":round(100*active/len(items),1),"is_product_completion_percent":False},
        "laboratory_evidence":{"claims":len(known),"distinct_runs":len({x["run_id"] for x in known.values()})}
    }

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--json",action="store_true")
    a=p.parse_args()
    data=json.loads((ROOT/"config/development-progress.json").read_text())
    release=json.loads((ROOT/"config/release-gates.json").read_text())
    lab=json.loads((ROOT/"config/lab-evidence.json").read_text())
    value=validate(data,release,lab)
    if a.json:
        print(json.dumps(value,ensure_ascii=False,indent=2))
    else:
        for label,row in value.items():
            frac=(f"{row['passed_levels']}/{row['total_levels']}" if 'passed_levels' in row else
                  f"{row['covered']}/{row['total']}" if 'covered' in row else
                  f"{row['certified']}/{row['total']}" if 'certified' in row else
                  f"{row['verified']}/{row['total']}" if 'verified' in row else
                  f"{row['claims']} proof jobs, {row['distinct_runs']} run IDs")
            percent=(f" {row['percent']}%" if "percent" in row else "")
            print(label+": "+frac+percent)
if __name__=="__main__":
    main()
