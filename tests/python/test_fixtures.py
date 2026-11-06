import copy
import json
import unittest
from tools.check_fixtures import check, validators
from tools.extract_protocol import ROOT, extract, run


class FixtureTests(unittest.TestCase):
    def test_extracted_data_is_exact(self):
        self.assertEqual(run(True)["fixtures"], 65)

    def test_standard_schema_expectations(self):
        self.assertEqual(check()["passed"], 65)

    def test_required_formats_are_active(self):
        checks = validators()
        fixtures = json.loads((ROOT / "fixtures/p0.1.json").read_text(encoding="utf-8"))
        event = copy.deepcopy(next(f["value"] for f in fixtures if f["id"] == "EV01"))
        event["received_at"] = "not-a-date"
        self.assertFalse(checks["event-v2"].is_valid(event))
        event["received_at"] = "2026-11-06T10:00:00Z"
        event["event_id"] = "not-a-uuid"
        self.assertFalse(checks["event-v2"].is_valid(event))

    def test_remote_schema_is_not_retrieved(self):
        from jsonschema import Draft202012Validator
        from referencing import Registry
        from tools.check_fixtures import no_retrieval
        check = Draft202012Validator({"$ref": "https://example.invalid/schema"},
                                     registry=Registry(retrieve=no_retrieval))
        with self.assertRaises(Exception):
            check.validate({})

    def test_virtual_tree_has_no_materialization_or_network(self):
        tree = json.loads((ROOT / "fixtures/virtual-tree.json").read_text(encoding="utf-8"))
        self.assertIs(tree["materialized"], False)
        self.assertIs(tree["real_network_enabled"], False)
        self.assertIs(tree["real_agent_connected"], False)


if __name__ == "__main__":
    unittest.main()
