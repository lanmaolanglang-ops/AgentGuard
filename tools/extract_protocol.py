"""Extract immutable P0.1 schemas and fixtures without executing embedded code."""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
BLOCK = re.compile(r"<!-- ag:(schema|fixture) ([^\n]*?) -->\s*\n~~~json\s*\n([\s\S]*?)\n~~~")


def extract(root):
    schemas, fixtures = {}, []
    for path in sorted((root / "docs/p0.1").glob("*.md")):
        text = path.read_text(encoding="utf-8")
        matches = list(BLOCK.finditer(text))
        if len(re.findall(r"^~~~json\s*$", text, re.M)) != len(matches):
            raise ValueError("存在未标记JSON " + path.name)
        for match in matches:
            kind, label, raw = match.groups()
            value = json.loads(raw)
            source = path.relative_to(root).as_posix()
            if kind == "schema":
                if label in schemas:
                    raise ValueError("重复Schema")
                schemas[label] = value
            else:
                identity, schema, expectation = label.split()
                if expectation not in ("valid", "invalid"):
                    raise ValueError("未知夹具预期")
                fixtures.append({"id": identity, "schema": schema, "valid": expectation == "valid",
                                 "source": source, "value": value})
    if len(schemas) != 9 or len(fixtures) != 65 or len({f["id"] for f in fixtures}) != 65:
        raise ValueError("冻结Schema或夹具数量不符")
    if schemas["types-v2"]["$defs"] != schemas["event-v2"]["$defs"]:
        raise ValueError("共享类型不一致")
    return schemas, fixtures


def outputs(root):
    schemas, fixtures = extract(root)
    result = {
        f"protocol/schemas/{name}.json": json.dumps(value, ensure_ascii=False, indent=2) + "\n"
        for name, value in schemas.items()
    }
    result["fixtures/p0.1.json"] = json.dumps(fixtures, ensure_ascii=False, indent=2) + "\n"
    result["fixtures/virtual-tree.json"] = json.dumps({
        "record_kind": "synthetic_fixture", "materialized": False,
        "root": "D:\\GuardLab",
        "files": [{"path": "project/README.md", "content": "AgentGuard synthetic fixture\n"},
                  {"path": "project/reports/fake.txt", "content": "FAKE_DATA_NO_CREDENTIALS"}],
        "network_sink": "sink.invalid", "real_network_enabled": False,
        "real_agent_connected": False
    }, ensure_ascii=False, indent=2) + "\n"
    return result


def run(check):
    result = outputs(ROOT)
    for name, text in result.items():
        path = ROOT / name
        if check:
            if not path.is_file() or path.read_text(encoding="utf-8") != text:
                raise ValueError("提取副本与设计基线不一致 " + name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    return {"issue": "I02-A", "schemas": 9, "fixtures": 65, "positive": 37,
            "negative": 28, "virtual_tree_materialized": False, "check_only": check}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    print(json.dumps(run(args.check), ensure_ascii=False))
