"""Validate all extracted fixtures using Draft 2020-12 and offline resources."""
import argparse
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource
from tools.extract_protocol import ROOT


def no_retrieval(uri):
    raise ValueError("Schema外部检索已禁用")


def validators(root=ROOT):
    checker = FormatChecker()
    for name, bad in (("date-time", "not-a-time"), ("uuid", "bad"), ("ipv4", "999.1.1.1")):
        if checker.conforms(bad, name):
            raise ValueError("必要format检查器不可用 " + name)
    schemas = {p.stem: json.loads(p.read_text(encoding="utf-8"))
               for p in (root / "protocol/schemas").glob("*.json")}
    registry = Registry(retrieve=no_retrieval)
    for schema in schemas.values():
        Draft202012Validator.check_schema(schema)
        registry = registry.with_resource(schema["$id"], Resource.from_contents(schema))
    return {name: Draft202012Validator(schema, registry=registry, format_checker=checker)
            for name, schema in schemas.items()}


def check(root=ROOT):
    checks = validators(root)
    fixtures = json.loads((root / "fixtures/p0.1.json").read_text(encoding="utf-8"))
    results = []
    for fixture in fixtures:
        errors = list(checks[fixture["schema"]].iter_errors(fixture["value"]))
        valid = not errors
        results.append({
            "id": fixture["id"], "schema": fixture["schema"],
            "expected_valid": fixture["valid"], "actual_valid": valid,
            "passed": valid == fixture["valid"],
            "errors": [{"instance_path": list(e.path), "schema_path": list(e.schema_path),
                        "keyword": e.validator} for e in errors[:4]],
        })
    report = {"schemas": len(checks), "fixtures": len(fixtures),
              "passed": sum(r["passed"] for r in results),
              "failed": sum(not r["passed"] for r in results), "results": results}
    if report["failed"]:
        raise ValueError("冻结Schema夹具结果不符")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = check()
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "results"}))
