"""One reproducible local/CI entry point. All subprocesses have deadlines."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def run():
    output = ROOT / ".work/ci"
    output.mkdir(parents=True, exist_ok=True)
    for name, command, expected in [
        ("rustc", ["rustc", "--version"], "1.97.1"),
        ("node", ["node", "--version"], "24.19.0"),
    ]:
        value = subprocess.check_output(command, cwd=ROOT, encoding="utf-8").strip()
        if expected not in value:
            raise RuntimeError("工具版本与I20-A不符 " + name)
    if sys.version_info[:3] != (3, 14, 6):
        raise RuntimeError("需要固定Python 3.14.6")
    commands = [
        ("baseline", [sys.executable, "-B", "tools/baseline.py"]),
        ("extraction", [sys.executable, "-B", "tools/extract_protocol.py", "--check"]),
        ("python-tests", [sys.executable, "-B", "-m", "unittest", "discover", "-s", "tests/python", "-v"]),
        ("schema-fixtures", [sys.executable, "-B", "-m", "tools.check_fixtures"]),
        ("crypto-references", [sys.executable, "-B", "-m", "tools.check_crypto"]),
        ("rust-format", ["cargo", "fmt", "--all", "--", "--check"]),
        ("rust-lint", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
        ("rust-build", ["cargo", "build", "--workspace", "--locked"]),
        ("rust-tests", ["cargo", "test", "--workspace", "--locked"]),
        ("windows-ipc", [sys.executable, "-B", "tools/test_ipc.py"]),
    ]
    results = []
    environment = dict(os.environ, PYTHONUTF8="1")
    for name, command in commands:
        started = time.monotonic()
        completed = subprocess.run(command, cwd=ROOT, capture_output=True,
                                   encoding="utf-8", errors="replace", timeout=900, env=environment)
        text = completed.stdout + "\n" + completed.stderr
        (output / (name + ".log")).write_text(text, encoding="utf-8")
        result = {"name": name, "command": command, "exit_code": completed.returncode,
                  "elapsed_seconds": round(time.monotonic() - started, 3)}
        results.append(result)
        print(json.dumps(result), flush=True)
        if completed.returncode:
            print(text[-12000:])
            break
    report = {"scope": "P1-A_first_batch_synthetic", "checks": results,
              "passed": sum(r["exit_code"] == 0 for r in results),
              "failed": sum(r["exit_code"] != 0 for r in results),
              "all_expected_checks_ran": len(results) == len(commands),
              "real_agents_at_L0": 7, "real_agent_tests_run": False,
              "isolation_certified": False}
    (output / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report["failed"] == 0 and report["all_expected_checks_ran"]


if __name__ == "__main__":
    sys.exit(0 if run() else 1)
