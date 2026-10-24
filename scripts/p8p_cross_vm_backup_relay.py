#!/usr/bin/env python3
"""A bounded one-use HTTP relay for a synthetic HAOS backup between disposable VMs."""
import argparse, hashlib, json, os, re
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

CAP=1024*1024*1024
def serve(directory,port):
    directory.mkdir(parents=True,exist_ok=True)
    directory.chmod(0o700)
    tar=directory/"archive.tar"
    metadata=directory/"manifest.json"
    class Handler(BaseHTTPRequestHandler):
        def log_message(self,*_): pass
        def fail(self,status):
            self.send_response(status)
            self.send_header("Content-Length","0")
            self.end_headers()
        def do_PUT(self):
            try: size=int(self.headers.get("Content-Length",""))
            except ValueError: return self.fail(400)
            slug=self.headers.get("X-Hars-Slug","")
            boot=self.headers.get("X-Hars-Boot","")
            version=self.headers.get("X-Hars-Core","")
            if tar.exists() or metadata.exists():
                return self.fail(409)
            if (self.path!="/archive"
                or not 1024<size<=CAP or not re.fullmatch("[0-9a-f]{8,16}",slug)
                or not re.fullmatch("[0-9a-f-]{36}",boot)
                or not re.fullmatch("[a-zA-Z0-9_.+-]{1,64}",version)):
                return self.fail(400)
            path=directory/"archive.partial"
            fd=None
            try:
                fd=os.open(path,os.O_EXCL|os.O_CREAT|os.O_WRONLY|os.O_NOFOLLOW,0o600)
                digest=hashlib.sha256()
                remaining=size
                while remaining:
                    block=self.rfile.read(min(remaining,1024*1024))
                    if not block: raise ValueError("truncated upload")
                    digest.update(block)
                    written=0
                    while written<len(block):
                        written+=os.write(fd,block[written:])
                    remaining-=len(block)
                os.fsync(fd);os.close(fd);fd=None
                os.replace(path,tar)
                body={"slug":slug,"source_boot_id":boot,"core_version":version,
                      "sha256":digest.hexdigest(),"size_bytes":size}
                with (directory/"manifest.partial").open("x") as f:
                    json.dump(body,f,sort_keys=True);f.write(chr(10))
                    f.flush();os.fsync(f.fileno())
                os.replace(directory/"manifest.partial",metadata)
                self.fail(201)
            except (OSError,ValueError):
                if fd is not None:os.close(fd)
                path.unlink(missing_ok=True)
                self.fail(400)
        def do_GET(self):
            if not metadata.is_file():return self.fail(404)
            if self.path=="/manifest":
                content=metadata.read_bytes()
                self.send_response(200)
                self.send_header("Content-Type","application/json")
                self.send_header("Content-Length",str(len(content)))
                self.end_headers()
                self.wfile.write(content)
            elif self.path=="/archive":
                fd=os.open(tar,os.O_RDONLY|os.O_NOFOLLOW)
                try:
                    size=os.fstat(fd).st_size
                    if not 1024<size<=CAP:return self.fail(500)
                    self.send_response(200)
                    self.send_header("Content-Length",str(size))
                    self.end_headers()
                    while True:
                        chunk=os.read(fd,1024*1024)
                        if not chunk:break
                        self.wfile.write(chunk)
                finally:os.close(fd)
            else:self.fail(404)
    http=HTTPServer(("127.0.0.1",port),Handler)
    try:http.serve_forever()
    finally:http.server_close()
if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--output-dir",type=Path,required=True)
    parser.add_argument("--port",type=int,default=18674)
    args=parser.parse_args()
    serve(args.output_dir,args.port)
