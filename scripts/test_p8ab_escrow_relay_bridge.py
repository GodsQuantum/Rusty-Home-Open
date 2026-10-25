#!/usr/bin/env python3
"""Move real disposable HAOS backup through the Rust durable opaque vault.
Only public hash, byte count and success markers are printed. No archive
content/secret values ever logged, uploaded as artifacts, or parsed.
"""
import argparse,hashlib,json
from pathlib import Path
import subprocess
ap=argparse.ArgumentParser()
ap.add_argument("--source-dir",type=Path,required=True)
ap.add_argument("--escrow-dir",type=Path,required=True)
ap.add_argument("--export-dir",type=Path,required=True)
ap.add_argument("--binary",type=Path,required=True)
args=ap.parse_args()
source=args.source_dir.resolve(strict=True)
archive=source/"archive.tar"
metadata=source/"manifest.json"
meta=json.loads(metadata.read_text())
with archive.open("rb") as f:
    assert hashlib.file_digest(f,"sha256").hexdigest()==meta["sha256"]
assert archive.stat().st_size==meta["size_bytes"]
escrow=args.escrow_dir
dest=args.export_dir
escrow.mkdir(mode=0o700)
dest.mkdir(mode=0o700)
def run(*opts):
    proc=subprocess.run([str(args.binary),"--lab-readonly",str(escrow),*map(str,opts)],
                        text=True,capture_output=True,timeout=60)
    assert proc.returncode==0,proc.stderr
    return json.loads(proc.stdout)
receipt=run("escrow-ingest",archive)
assert receipt["sha256"]==meta["sha256"] and receipt["bytes"]==meta["size_bytes"]
assert receipt["byte_exact"] and not receipt["production_ready"]
repeated=run("escrow-ingest",archive)
assert repeated==receipt
result=run("escrow-export",receipt["sha256"],dest)
assert result==receipt
with (dest/"archive.tar").open("rb") as f:
    assert hashlib.file_digest(f,"sha256").hexdigest()==meta["sha256"]
assert (dest/"archive.tar").stat().st_size==meta["size_bytes"]
with (dest/"manifest.json").open("x") as f:
    f.write(json.dumps(meta,sort_keys=True)+"\n")
    f.flush()
    __import__("os").fsync(f.fileno())
(dest/"manifest.json").chmod(0o600)
# Remove the original guest relay archive. The fresh second VM MUST fetch
# the file exported by native Rust, rather than accidentally using it.
archive.unlink()
__import__("os").replace(dest/"archive.tar",archive)
with archive.open("rb") as stream:
    assert hashlib.file_digest(stream,"sha256").hexdigest()==meta["sha256"]
assert (escrow/(receipt["sha256"]+".tar")).is_file()
print("P8AB_REAL_HAOS_FULL_ARCHIVE_THROUGH_NATIVE_RUST_ESCROW_PASS")
