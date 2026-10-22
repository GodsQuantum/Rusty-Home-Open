#!/usr/bin/env python3
"""Tests for bounded, line-safe HAOS jq chunk transport of real Supervisor JSON."""
import base64
import json
import unittest
from p8c2_store_codec import decode_chunked_store

class RealStoreConsoleCodecTests(unittest.TestCase):
    def setUp(self):
        self.payload={"result":"ok","data":{"addons":[
            {"slug":f"unknown_{n}", "name":f"Device à {n} 🚀"} for n in range(1200)
        ],"repositories":[{"slug":"core","name":"Official"}]}}
        self.text=json.dumps(self.payload,ensure_ascii=False,separators=(",",":"))
        self.assertGreater(len(self.text),20520)
        self.chunks=[
            base64.b64encode(self.text[i:i+2048].encode()).decode("ascii")
            for i in range(0,len(self.text),2048)
        ]

    def test_1200_unknown_apps_survive_real_jq_chunk_transport(self):
        result=decode_chunked_store(self.chunks,len(self.text))
        self.assertEqual(result,self.payload)

    def test_missing_truncated_corrupted_and_spurious_chunks_denied(self):
        for chunks in (self.chunks[:-1],self.chunks+["AAAA"],["%bad%"],[]):
            with self.assertRaises(ValueError):
                decode_chunked_store(chunks,len(self.text))

    def test_prevents_overlong_declared_store_size(self):
        with self.assertRaises(ValueError):
            decode_chunked_store(self.chunks,8*1024*1024+1)

if __name__ == "__main__":
    unittest.main()
