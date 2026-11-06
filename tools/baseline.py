"""Read-only integrity, inventory and dependency checks for I01 and later nodes."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
FLAGS = (
    "telemetry_verified",
    "pre_execution_block_verified",
    "parameter_binding_verified",
    "single_use_grant_verified",
    "os_isolation_verified",
)
NAMES = (
    "Codex", "Claude Code", "DeepSeek Harness（DSH）",
    "豆包 App 内工作模式", "Reasonix", "WorkBuddy", "ZCode",
)
GRADES = ("E1", "E1", "E2", "E0", "E2", "E1", "E1")


def check_design(root, manifest):
    files = manifest["files"]
    expected = {entry["path"] for entry in files}
    actual = {
        path.relative_to(root).as_posix()
        for folder in ("p0", "p0.1")
        for path in (root / "docs" / folder).rglob("*")
        if path.is_file()
    }
    if len(files) != 42 or len(expected) != 42 or expected != actual:
        raise ValueError("设计基线文件清单发生变化")
    for entry in files:
        raw = (root / entry["path"]).read_bytes()
        if len(raw) != entry["bytes"] or hashlib.sha256(raw).hexdigest() != entry["sha256"]:
            raise ValueError("设计基线字节发生变化 " + entry["path"])
    return len(files)


def check_inventory(document):
    agents = document["agents"]
    if len(agents) != 7:
        raise ValueError("Agent范围必须保留七款")
    for index, agent in enumerate(agents):
        if (agent["agent_id"], agent["name"]) != (f"A0{index + 1}", NAMES[index]):
            raise ValueError("Agent身份或顺序不匹配")
        if agent["level"] != "L0" or set(agent["capabilities"]) != set(FLAGS):
            raise ValueError("当前范围只能显示L0与五个独立标志")
        if any(value is not False for value in agent["capabilities"].values()):
            raise ValueError("合成工程不能提高真实Agent能力")
        if agent["historical_evidence_grade"] != GRADES[index]:
            raise ValueError("历史Agent证据不能升级")
        if agent["execution_modes_verified"]:
            raise ValueError("没有真实执行模式认证")
        for key in ("client_version", "runtime_version", "tool_provider", "config_digest",
                    "binary", "publisher_signature", "source_refs", "unknowns"):
            if key not in agent:
                raise ValueError("身份缺口必须显式保留 " + key)
        if not agent["source_refs"] or not agent["unknowns"]:
            raise ValueError("证据来源和未知范围不可省略")
    return len(agents)


def check_graph(text):
    tasks = {}
    for line in re.findall(r"^\| I\d{2}[^\n]+$", text, re.M):
        parts = [part.strip() for part in line.strip("|").split("|")]
        if len(parts) != 7 or not all(parts):
            raise ValueError("任务交付和验收字段不完整")
        node, phase, owner, title, dependencies, deliverable, acceptance = parts
        if node in tasks:
            raise ValueError("重复任务")
        tasks[node] = [] if dependencies == "无" else dependencies.split(", ")
    edges = {(before.replace("-", "_"), node.replace("-", "_"))
             for node, dependencies in tasks.items() for before in dependencies}
    graph = re.search(r"<!-- ag:p1-dag -->\s*\n~~~mermaid\s*\n([\s\S]*?)\n~~~", text)
    if graph is None:
        raise ValueError("缺少依赖图")
    drawn = re.findall(r"^\s*(I\w+)\s+-->\s+(I\w+)\s*$", graph.group(1), re.M)
    if len(tasks) != 27 or len(edges) != 117 or len(drawn) != len(set(drawn)):
        raise ValueError("执行节点或依赖数量变化")
    if set(drawn) != edges or {node[:3] for node in tasks} != {f"I{i:02}" for i in range(1, 25)}:
        raise ValueError("图表依赖不一致")
    remaining = {node: set(deps) for node, deps in tasks.items()}
    order = []
    while remaining:
        ready = sorted(node for node, deps in remaining.items() if not deps)
        if not ready:
            raise ValueError("任务存在循环或未知前置节点")
        order.extend(ready)
        for node in ready:
            del remaining[node]
        for deps in remaining.values():
            deps.difference_update(ready)
    return {"parents": 24, "nodes": len(tasks), "edges": len(edges), "order": order}


def run(root=ROOT):
    evidence = root / "docs" / "p1-a" / "evidence" / "I01"
    manifest = json.loads((evidence / "design-baseline.json").read_text(encoding="utf-8"))
    inventory = json.loads((evidence / "agent-inventory.json").read_text(encoding="utf-8"))
    return {
        "issue": "I01",
        "design_files_unchanged": check_design(root, manifest),
        "agents_at_L0": check_inventory(inventory),
        "dependency_graph": check_graph((root / "docs/p0.1/14-P1-DEVELOPMENT-PLAN.md").read_text(encoding="utf-8")),
    }


if __name__ == "__main__":
    print(json.dumps(run(), ensure_ascii=False, indent=2))
