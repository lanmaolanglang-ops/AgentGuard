import copy
import json
from pathlib import Path
import unittest

from tools.baseline import ROOT, check_design, check_graph, check_inventory

EVIDENCE = ROOT / "docs/p1-a/evidence/I01"


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.inventory = json.loads((EVIDENCE / "agent-inventory.json").read_text(encoding="utf-8"))

    def test_seven_agents_remain_L0(self):
        self.assertEqual(check_inventory(self.inventory), 7)

    def test_true_capability_rejected(self):
        self.inventory["agents"][0]["capabilities"]["os_isolation_verified"] = True
        with self.assertRaises(ValueError):
            check_inventory(self.inventory)

    def test_extra_agent_rejected(self):
        self.inventory["agents"].append(copy.deepcopy(self.inventory["agents"][0]))
        with self.assertRaises(ValueError):
            check_inventory(self.inventory)

    def test_workbuddy_identity_cannot_be_codebuddy(self):
        self.inventory["agents"][5]["name"] = "CodeBuddy"
        with self.assertRaises(ValueError):
            check_inventory(self.inventory)

    def test_evidence_upgrade_rejected(self):
        self.inventory["agents"][0]["historical_evidence_grade"] = "E3"
        with self.assertRaises(ValueError):
            check_inventory(self.inventory)

    def test_missing_unknown_field_rejected(self):
        del self.inventory["agents"][3]["config_digest"]
        with self.assertRaises(ValueError):
            check_inventory(self.inventory)

    def test_design_hashes_and_tampered_manifest(self):
        manifest = json.loads((EVIDENCE / "design-baseline.json").read_text(encoding="utf-8"))
        self.assertEqual(check_design(ROOT, manifest), 42)
        manifest["files"][0]["sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            check_design(ROOT, manifest)

    def test_dependency_graph_matches_and_changes_rejected(self):
        text = (ROOT / "docs/p0.1/14-P1-DEVELOPMENT-PLAN.md").read_text(encoding="utf-8")
        result = check_graph(text)
        self.assertEqual((result["parents"], result["nodes"], result["edges"]), (24, 27, 117))
        with self.assertRaises(ValueError):
            check_graph(text.replace("  I01 --> I03\n", ""))


if __name__ == "__main__":
    unittest.main()
