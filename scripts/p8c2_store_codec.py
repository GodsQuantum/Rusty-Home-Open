"""Bounded, short-line transport of actual HAOS Supervisor Store JSON.

HAOS's host shell includes jq but not gzip. Real JSON one-line responses
can be truncated by the QEMU/labgrid console near 20 KiB. The HAOS guest
stores the CLI response only on its disposable filesystem. The runner reads
short jq base64-encoded chunks using repeated ShellDriver requests.
"""
from __future__ import annotations
import base64
import binascii
import json

MAX_JSON_CHARS = 8 * 1024 * 1024
CHUNK_CHARS = 2048
MAX_BASE64_LINE = 11200

def decode_chunked_store(encoded_chunks: list[str], expected_chars: int) -> dict:
    try:
        if not isinstance(expected_chars, int) or not (0 < expected_chars <= MAX_JSON_CHARS):
            raise ValueError("invalid declared size")
        count=(expected_chars + CHUNK_CHARS - 1)//CHUNK_CHARS
        if len(encoded_chunks) != count:
            raise ValueError("missing, duplicate, or extra chunk")
        parts=[]
        for i, encoded in enumerate(encoded_chunks):
            if not encoded or len(encoded)>MAX_BASE64_LINE:
                raise ValueError("invalid chunk size")
            data=base64.b64decode(encoded, validate=True)
            fragment=data.decode("utf8")
            want = min(CHUNK_CHARS, expected_chars-i*CHUNK_CHARS)
            if len(fragment)!=want:
                raise ValueError("truncated chunk")
            parts.append(fragment)
        text="".join(parts)
        if len(text)!=expected_chars:
            raise ValueError("wrong final length")
        result=json.loads(text)
        if not isinstance(result,dict) or result.get("result")!="ok":
            raise ValueError("Supervisor Store API did not return success")
        return result
    except (UnicodeError, ValueError, binascii.Error, json.JSONDecodeError):
        raise ValueError("invalid bounded Supervisor Store JSON chunk transport") from None
