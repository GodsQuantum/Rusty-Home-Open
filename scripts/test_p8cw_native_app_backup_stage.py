#!/usr/bin/env python3
"""P8cw native Rust app-only Supervisor archive admission, hash escrow and denial.

All synthetic files. No actual App install, archive extraction, execution,
household data or user-owned production service.
"""
import argparse,hashlib,io,json,os,subprocess,tarfile,tempfile
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument("--binary",type=Path,required=True)
a=p.parse_args()
binary=a.binary.resolve(strict=True)

def part(*, duplicate=False, symlink=False, traversal=False):
    buf=io.BytesIO()
    with tarfile.open(fileobj=buf,mode="w:gz") as tf:
        def file(name,body,*,kind=tarfile.REGTYPE):
            hdr=tarfile.TarInfo("./"+name)
            hdr.type=kind
            hdr.size=len(body) if kind==tarfile.REGTYPE else 0
            if kind==tarfile.SYMTYPE: hdr.linkname="/etc/passwd"
            tf.addfile(hdr,io.BytesIO(body) if kind==tarfile.REGTYPE else None)
        name="../bad" if traversal else "data/synthetic-fixture.txt"
        file(name,b"synthetic p8cw private archived App data\n",kind=tarfile.SYMTYPE if symlink else tarfile.REGTYPE)
        if duplicate: file(name,b"duplicated in same tar\n")
    return buf.getvalue()

def outer(path,*,protected=False,kind="partial",manifest_addons=("core_configurator",),
          tar_parts=("core_configurator.tar.gz",),payload=None,other=(),bad_manifest=False):
    manifest={"type":kind,"protected":protected,"addons":list(manifest_addons)}
    with tarfile.open(path,"w") as tf:
        def append(name,contents):
            h=tarfile.TarInfo("./"+name)
            h.size=len(contents)
            tf.addfile(h,io.BytesIO(contents))
        raw=json.dumps(manifest).encode()
        if bad_manifest: raw=b'{"type":"partial","protected":false,"protected":true,"addons":["core_configurator"]}'
        append("backup.json",raw)
        for filename in tar_parts: append(filename,payload if payload is not None else part())
        for filename,data in other: append(filename,data)
    path.chmod(0o600)

def run(private,*args,ok=True):
    result=subprocess.run([str(binary),"--lab-readonly",str(private),*map(str,args)],
                          capture_output=True,text=True,timeout=35)
    if ok:
        assert result.returncode==0,(args,result.stderr)
        got=json.loads(result.stdout)
        assert "synthetic p8cw" not in result.stdout
        assert got["production_ready"] is False
        assert got["apps_restored"] is False
        assert got["actions_executable"] is False
        return got
    assert result.returncode!=0,(args,result.stdout)
    assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in result.stderr
    assert not result.stdout

with tempfile.TemporaryDirectory(prefix="p8cw-archive-test-") as temp:
    root=Path(temp)
    private=root/"escrow"
    private.mkdir(mode=0o700)
    src=root/"source.tar"
    outer(src)
    digest=hashlib.sha256(src.read_bytes()).hexdigest()
    first=run(private,"app-source-ingest",src)
    assert first["sha256"]==digest and first["app_count"]==1
    assert first["files"]==1
    assert first["bytes"]==src.stat().st_size
    assert first["declared_payload_bytes"]==len(b"synthetic p8cw private archived App data\n")
    assert run(private,"app-source-verify",digest)["archive_sha256"]==digest
    assert run(private,"app-source-ingest",src)["sha256"]==digest
    assert len(list(private.glob("*.tar")))==1
    assert (private/(digest+".tar")).stat().st_mode & 0o077==0
    print("P8CW_RUST_NATIVE_APP_ONLY_PRIVATE_SHA_ESCROW_REOPEN_AND_IDEMPOTENCY_PASS")
    for name,mutations in [
        ("encrypted",dict(protected=True)),
        ("full",dict(kind="full")),
        ("missing",dict(tar_parts=())),
        ("undeclared",dict(tar_parts=("unexpected.tar.gz",))),
        ("other_content",dict(other=(("homeassistant.tar.gz",part()),))),
        ("invalid_json_duplicates",dict(bad_manifest=True)),
        ("bad_slug",dict(manifest_addons=("../escape",))),
        ("duplicated_declared_app",dict(manifest_addons=("core_configurator","core_configurator"))),
        ("inner_symlink",dict(payload=part(symlink=True))),
        ("inner_traversal",dict(payload=part(traversal=True))),
        ("inner_duplicate",dict(payload=part(duplicate=True))),
        ("truncated_gzip",dict(payload=part()[:53])),
        ("outer_duplicate",dict(other=(("backup.json",b"{}"),))),
    ]:
        outer(src,**mutations)
        run(private,"app-source-ingest",src,ok=False)
        assert len(list(private.glob("*.tar")))==1,name
    outer(src)
    src.unlink()
    run(private,"app-source-ingest",src,ok=False)
    snapshot=private/(digest+".tar")
    raw=snapshot.read_bytes()
    snapshot.write_bytes(raw[:1024]+b"corrupted"+raw[1033:])
    run(private,"app-source-verify",digest,ok=False)
    print("P8CW_RUST_NATIVE_APP_METADATA_GZIP_TRAVERSAL_DUPLICATES_AND_TAMPER_DENIAL_PASS")
