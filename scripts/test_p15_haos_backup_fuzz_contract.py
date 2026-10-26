#!/usr/bin/env python3
"""Scoped fifth libFuzzer target: genuine Rust importer, invented archives."""
from pathlib import Path
import io
import tarfile
import tomllib

root=Path(__file__).resolve().parent.parent
source=(root/"fuzz/fuzz_targets/haos_backup_import.rs").read_text()
cargo=tomllib.loads((root/"fuzz/Cargo.toml").read_text())
workflow=(root/".github/workflows/fuzz.yml").read_text()
assert any(b["name"]=="haos_backup_import" for b in cargo["bin"])
assert "hars-interop-moli" in cargo["dependencies"]
assert "registry_from_unprotected_backup(&archive)" in source
assert "parse_export(&result.raw)" in source
assert "result.automations_yaml" in source
for banned in ("TcpListener","TcpStream","SUPERVISOR_TOKEN","VM100","/etc/passwd"):
    assert banned not in source,banned
for token in ("fuzz run haos_backup_import","-max_total_time=45",
              "-max_total_time=180","fuzz/artifacts/haos_backup_import/",
              "crates/interop-moli/src/ha_backup_import.rs"):
    assert token in workflow,token
folder=root/"fuzz/corpus/haos_backup_import"
seeds=list(folder.glob("*.bin"))
assert len(seeds)==5
assert sorted(p.name for p in seeds)==[
    "synthetic-duplicate-refused.bin","synthetic-invalid-utf8-refused.bin",
    "synthetic-protected-refused.bin","synthetic-traversal-refused.bin",
    "synthetic-valid-full.bin"]
for seed in seeds:
    data=seed.read_bytes()
    assert 1024<=len(data)<65536
    assert b"real-person" not in data and b"SUPERVISOR_TOKEN" not in data
    with tarfile.open(fileobj=io.BytesIO(data),mode="r:") as archive:
        members=archive.getmembers()
        assert any(x.name=="backup.json" for x in members)
        assert any(x.name=="homeassistant.tar.gz" for x in members)
assert "/corpus/haos_backup_import/*" in (root/"fuzz/.gitignore").read_text()
assert "fuzz/corpus/haos_backup_import/*.bin binary" in (root/".gitattributes").read_text()
print("P15_REAL_HAOS_BACKUP_IMPORTER_FUZZ_SCOPE_AND_SYNTHETIC_CORPUS_PASS")
