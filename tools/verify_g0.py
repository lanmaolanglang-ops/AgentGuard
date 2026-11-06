"""Run G0 gates in order, preserving evidence before advancing to the next gate."""

import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / ".work" / "g0"
BASE = "6f9bcf50ce5b327dc0225f612b5982990fe1716a"
SQLITE_SHA256 = "0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def command(args, *, cwd=ROOT, timeout=300):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True,
                          encoding="utf-8", timeout=timeout)


def stage(name, script):
    result = command([sys.executable, "-B", script], timeout=360)
    (EVIDENCE / f"{name}.log").write_text(result.stdout + "\n" + result.stderr, encoding="utf-8")
    require(result.returncode == 0, f"{name} failed; inspect {name}.log")
    return result.stdout


def schema_evidence():
    metadata = command(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
                       cwd=ROOT / "crates/ledger")
    require(metadata.returncode == 0, "cannot verify the SQLite amalgamation")
    package, = [p for p in json.loads(metadata.stdout)["packages"] if p["name"] == "libsqlite3-sys"]
    require(package["version"] == "0.39.1", "unexpected SQLite crate")
    amalgamation = Path(package["manifest_path"]).parent / "sqlite3/sqlite3.c"
    actual_sha256 = hashlib.sha256(amalgamation.read_bytes()).hexdigest()
    require(actual_sha256 == SQLITE_SHA256, "SQLite source differs from accepted C2")
    migration = ROOT / "crates/ledger/migrations"
    before = b"\n".join((migration / name).read_bytes() for name in ("001-grants.sql", "002-events.sql"))
    after = before + b"\n" + (migration / "003-g0-integrity.sql").read_bytes()
    (EVIDENCE / "schema-before.sql").write_bytes(before)
    (EVIDENCE / "schema-after.sql").write_bytes(after)
    diff = command(["git", "diff", BASE, "--", "crates/ledger/src/db.rs", "crates/ledger/src/store.rs"])
    require(diff.returncode == 0 and diff.stdout, "missing Rust writer diff")
    (EVIDENCE / "rust-writer.diff").write_text(diff.stdout, encoding="utf-8")
    frozen = command(["git", "diff", "--exit-code", BASE, "--", "docs/p0", "docs/p0.1",
                      "protocol/schemas", "probes/i05-c2", "crates/ledger/migrations/001-grants.sql",
                      "crates/ledger/migrations/002-events.sql", "crates/ledger/Cargo.lock",
                      "crates/ledger/.cargo/config.toml", "docs/p1-a/I05-LEDGER-REPORT.md",
                      "docs/p1-a/I05-C2-ACCEPTANCE.md"])
    require(frozen.returncode == 0, "a protected input differs from I05")
    shutil.copyfile(ROOT / "docs/p1-a/evidence/I01/design-baseline.json", EVIDENCE / "frozen-design.json")
    return {"sqlite_amalgamation_sha256": actual_sha256,
            "schema_before_sha256": hashlib.sha256(before).hexdigest(),
            "schema_after_sha256": hashlib.sha256(after).hexdigest()}


def main():
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    ledger_output = stage("ledger", "tools/verify_i05_ledger.py")
    ledger = json.loads((ROOT / ".work/i05-ledger/evidence.json").read_text(encoding="utf-8"))
    expected = {f"G0{group}-{i:02}" for group, total in (("a", 10), ("b", 12), ("c", 14))
                for i in range(1, total + 1)}
    observed = set(re.findall(r"(?m)^EVIDENCE (G0[abc]-\d\d) ", ledger_output))
    require(expected == observed, f"G0 evidence missing or unexpected: {sorted(expected ^ observed)}")
    require(ledger["passed"] == 27 and ledger["failed"] == 0, "wrong ledger regression count")
    require(sum(" fault=" in line for line in ledger["observations"]) == 7, "missing crash point")
    require("recursive=1 trusted_schema=0" in ledger_output and "database_pairs=64 legal=15 rejected=49" in ledger_output,
            "missing SQLite configuration or transition matrix evidence")
    require("G0-migration 2->3" in ledger_output and "G0-migration-invalid" in ledger_output
            and "G0-before version=3.54.1" in ledger_output, "missing migration/baseline evidence")
    shutil.copyfile(ROOT / ".work/i05-ledger/evidence.json", EVIDENCE / "ledger-evidence.json")
    source = schema_evidence()

    c2 = json.loads(stage("c2", "probes/i05-c2/run.py"))
    require(c2["result"] == "PASS" and c2["observations"]["sqlite_runtime"]["version"] == "3.54.1"
            and c2["build"]["sqlite_amalgamation_sha3_256"] == ledger["sqlite_amalgamation_sha3_256"],
            "C2 or exact SQLite source regression failed")
    (EVIDENCE / "c2-evidence.json").write_text(json.dumps(c2, indent=2) + "\n", encoding="utf-8")

    stage("first-batch", "tools/ci.py")
    baseline_path = ROOT / ".work/ci"
    baseline = json.loads((baseline_path / "summary.json").read_text(encoding="utf-8"))
    require(baseline["passed"] == 10 and baseline["failed"] == 0 and baseline["all_expected_checks_ran"],
            "first-batch gate regression failed")
    for name, expected_count in (("python-tests", "Ran 17 tests"), ("rust-tests", "14 passed; 0 failed"),
                                 ("windows-ipc", "Ran 13 tests")):
        require(expected_count in (baseline_path / f"{name}.log").read_text(encoding="utf-8"),
                f"missing baseline test count for {name}")
    shutil.copytree(baseline_path, EVIDENCE / "first-batch", dirs_exist_ok=True)
    summary = {
        "scope": "G0 synthetic engineering evidence",
        "head": ledger["head"],
        "ledger_passed": ledger["passed"],
        "ledger_failed": 0,
        "original_i05_tests": 19,
        "g0_additional_test_functions": 8,
        "g0_cases_observed": sorted(observed),
        "c2": c2["result"],
        "first_batch_gates_passed": 10,
        "first_batch_tests_passed": 44,
        "real_agents_at_l0": 7,
        **source,
    }
    (EVIDENCE / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
