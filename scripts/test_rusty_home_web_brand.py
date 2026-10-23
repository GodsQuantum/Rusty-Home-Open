#!/usr/bin/env python3
"""Rusty Home visible branding without pretending that deployment is certified."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
class BrandIdentity(unittest.TestCase):
    def test_visible_html_uses_canonical_name_and_lab_notice(self):
        html=(ROOT/"web/index.html").read_text()
        self.assertIn("<title>Rusty Home",html)
        self.assertIn("<h1>Rusty Home</h1>",html)
        self.assertNotIn("Home Automation Rust",html)
        self.assertIn("Prototype",html)
        self.assertIn("lecture seule",html)
        self.assertIn("migration complète ne sont pas encore disponibles",html)
    def test_smoke_validates_server_render_not_just_source(self):
        script=(ROOT/"scripts/smoke-http.sh").read_text()
        self.assertIn('http://127.0.0.1:18790/',script)
        self.assertIn('<h1>Rusty Home</h1>',script)
        self.assertIn('r["release_gates_blocked"]==15',script)
        self.assertIn('"haos_apps_total" not in r',script)
if __name__=="__main__":
    unittest.main()
