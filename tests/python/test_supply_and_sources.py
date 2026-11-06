import json
import re
import unittest
from tools.extract_protocol import ROOT


class SourceAndSupplyTests(unittest.TestCase):
    def test_every_source_is_recorded_without_runtime_claims(self):
        path = ROOT / "docs/p1-a/evidence/I23/source-reachability.json"
        report = json.loads(path.read_text(encoding="utf-8"))
        sources = report["sources"]
        self.assertEqual(len(sources), 76)
        self.assertEqual(len({row["id"] for row in sources}), 76)
        self.assertTrue(all(row["runtime_verified"] is False for row in sources))
        self.assertEqual(sum(row.get("status") == 200 for row in sources), report["http_200"])

    def test_fixed_conflicts_keep_commit_and_hash(self):
        path = ROOT / "docs/p1-a/evidence/I23/fixed-source-excerpts.json"
        rows = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(len(rows), 3)
        for row in rows:
            self.assertEqual(row["commit"], "f4f32cc7d3078f7031c7852fbf0b7de410958199")
            self.assertRegex(row["sha256"], r"^[0-9a-f]{64}$")
            self.assertTrue(row["matched_lines"])
            self.assertIs(row["runtime_verified"], False)

    def test_locked_development_sbom_and_advisory_result(self):
        path = ROOT / "docs/p1-a/evidence/I20-A/development-sbom.json"
        report = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(len(report["rust_packages"]), 104)
        self.assertFalse(report["sqlite_dependency"])
        self.assertEqual(report["advisories"]["status"], "checked")
        self.assertEqual(report["advisories"]["findings"], [])
        for row in report["rust_packages"]:
            self.assertEqual(row["source"], "registry+https://github.com/rust-lang/crates.io-index")
            self.assertRegex(row["checksum"], r"^[0-9a-f]{64}$")
        tokio = next(row for row in report["rust_packages"] if row["name"] == "tokio")
        self.assertEqual(tokio["version"], "1.54.1")

    def test_python_wheel_hashes_are_pinned_in_lock(self):
        path = ROOT / "docs/p1-a/evidence/I20-A/python-package-sources.json"
        packages = json.loads(path.read_text(encoding="utf-8"))
        lock = (ROOT / "requirements-ci.lock").read_text(encoding="utf-8")
        self.assertEqual(len(packages), 7)
        for package in packages:
            self.assertIn(package["name"] + "==" + package["version"], lock)
            self.assertTrue(package["files"])
            for wheel in package["files"]:
                self.assertTrue(wheel["filename"].endswith(".whl"))
                self.assertTrue(wheel["url"].startswith("https://files.pythonhosted.org/"))
                self.assertIn("--hash=sha256:" + wheel["sha256"], lock)


if __name__ == "__main__":
    unittest.main()
