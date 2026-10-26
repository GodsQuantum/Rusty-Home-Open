#!/usr/bin/env python3
"""Real CNCF Distribution v2 blob upload/read via strictly localhost HTTP."""
import hashlib
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

BASE = "http://127.0.0.1:18884"
BLOB = b"rusty-home-generic-app-registry-blob-20261026"
DIGEST = "sha256:" + hashlib.sha256(BLOB).hexdigest()
TARGET = BASE + "/v2/rusty-home-test/blobs/" + DIGEST

def request(url,method="GET",data=None):
    assert url.startswith(BASE + "/v2/"),url
    req=urllib.request.Request(url,method=method,data=data)
    return urllib.request.urlopen(req,timeout=4)

for attempt in range(80):
    try:
        with request(BASE+"/v2/") as res:
            assert res.status==200
        break
    except (urllib.error.URLError,ConnectionError,TimeoutError):
        time.sleep(.25)
else:
    raise SystemExit("Official Registry OCI App failed to become ready")

if sys.argv[1:] == ["--upload"]:
    with request(BASE+"/v2/rusty-home-test/blobs/uploads/",method="POST",data=b"") as res:
        assert res.status==202,res.status
        location=res.headers["Location"]
    url=urllib.parse.urljoin(BASE,location)
    parts=urllib.parse.urlsplit(url)
    assert parts.scheme=="http" and parts.netloc=="127.0.0.1:18884"
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
