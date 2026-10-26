#!/usr/bin/env python3
"""Real CNCF Distribution v2 blob upload/read via strictly localhost HTTP."""
import hashlib
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

# Operator-selected LOCAL lab ports only. No outbound/external endpoints.
import os
PORT = os.environ.get("HARS_REGISTRY_LAB_PORT", "18884")
assert PORT in ("18884", "18885")
BASE = "http://127.0.0.1:" + PORT
BLOB = (
    b"rusty-home-after-backup-blob-20261026"
    if sys.argv[1:] in (["--upload-later"], ["--verify-later-absent"])
    else b"rusty-home-generic-app-registry-blob-20261026"
)
DIGEST = "sha256:" + hashlib.sha256(BLOB).hexdigest()
TARGET = BASE + "/v2/rusty-home-test/blobs/" + DIGEST

def request(url,method="GET",data=None):
    assert url.startswith(BASE + "/v2/"),url
    headers={"Content-Type":"application/octet-stream"} if data is not None else {}
    req=urllib.request.Request(url,method=method,data=data,headers=headers)
    try:
        return urllib.request.urlopen(req,timeout=4)
    except urllib.error.HTTPError as exc:
        # Controlled local-only test HTTP response; never log bearer tokens,
        # URLs with opaque upload state, or external destinations.
        print("P8AW_DISTRIBUTION_HTTP_FAILURE",method,exc.code,exc.read(256).decode("utf-8","replace"))
        raise

for attempt in range(80):
    try:
        with request(BASE+"/v2/") as res:
            assert res.status==200
        break
    except (urllib.error.URLError,ConnectionError,TimeoutError):
        time.sleep(.25)
else:
    raise SystemExit("Official Registry OCI App failed to become ready")

if sys.argv[1:] == ["--verify-later-absent"]:
    # A volume restored from an older checkpoint MUST NOT contain a blob
    # committed to the source after export, even if its digest is known.
    try:
        with request(TARGET):
            raise AssertionError("Restored Registry leaked post-backup blob")
    except urllib.error.HTTPError as exc:
        assert exc.code == 404, exc.code
    print("P8AX_RESTORED_REGISTRY_POST_CHECKPOINT_BLOB_ABSENT_PASS")
    raise SystemExit(0)
if sys.argv[1:] in (["--upload"], ["--upload-later"]):
    with request(BASE+"/v2/rusty-home-test/blobs/uploads/",method="POST",data=b"") as res:
        assert res.status==202,res.status
        location=res.headers["Location"]
    url=urllib.parse.urljoin(BASE,location)
    parts=urllib.parse.urlsplit(url)
    assert parts.scheme=="http" and parts.netloc=="127.0.0.1:"+PORT
    assert parts.path.startswith("/v2/rusty-home-test/blobs/uploads/")
    sep="&" if parts.query else "?"
    with request(url+sep+urllib.parse.urlencode({"digest":DIGEST}),method="PUT",data=BLOB) as res:
        assert res.status==201,res.status
elif sys.argv[1:] != ["--verify"]:
    raise SystemExit("Expected --upload or --verify")
with request(TARGET) as res:
    assert res.status==200
    assert res.read()==BLOB
print("P8AW_REAL_REGISTRY_HTTP_BLOB_UPLOAD_OR_VERIFY_PASS",sys.argv[1])
