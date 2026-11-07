"""Run the Windows S1 gates and package logs plus independent SQLite evidence."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import tomllib
import uuid
from zipfile import ZIP_DEFLATED, ZipFile


ROOT = Path(__file__).resolve().parents[1]
D_SHA = "4a1e900d71b3f197f2f2763816fa98800ce93feb"
G0_SHA = "8c2a026468e05b352e4e3ed9de1db3dcc3ce8743"
LEDGER = "crates/ledger"
TYPED = "crates/typed-authorization"
MIGRATIONS = ("001-grants.sql", "002-events.sql", "003-g0-integrity.sql", "004-s-atomic.sql")
FROZEN = ("docs/p0", "docs/p0.1", "protocol", "probes/i05-c2", "Cargo.lock", "requirements-ci.lock",
          f"{LEDGER}/.cargo/config.toml", f"{LEDGER}/src/tests.rs", f"{LEDGER}/src/g0_tests.rs",
          f"{TYPED}/src/tests.rs", f"{TYPED}/Cargo.lock", "tools/verify_i06_d.py", "tools/verify_g0.py",
          "tools/verify_i05_ledger.py", *[f"{LEDGER}/migrations/{file}" for file in MIGRATIONS[:3]])


def require(condition, detail):
    if not condition:
        raise RuntimeError(detail)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pins(content):
    return {(p["name"], p["version"], p["source"], p["checksum"])
            for p in tomllib.loads(content)["package"] if "source" in p}


def report_schema(out, observations):
    expected = {}
    with sqlite3.connect(":memory:") as db:
        for version, name in enumerate(MIGRATIONS, 1):
            db.executescript((ROOT / LEDGER / "migrations" / name).read_text(encoding="utf-8"))
            if version >= 3:
                objects = db.execute("SELECT type,name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY type,name").fetchall()
                expected[version] = {
                    "sha256": hashlib.sha256(json.dumps(objects, ensure_ascii=False).encode()).hexdigest(),
                    "objects": [{"type": kind, "name": name} for kind, name, _ in objects],
                }
    for path, facts in observations:
        require(facts["schema_sha256"] == expected[facts["version"]]["sha256"], f"schema drift in {path.name}")
        require(facts["snapshot_sha256"] == sha(path.with_suffix(".sqlite3")), f"snapshot mismatch {path.name}")
    report = {"reference": expected, "inspected_snapshots": len(observations),
              "migration_hashes": {name: sha(ROOT / LEDGER / "migrations" / name) for name in MIGRATIONS}}
    (out / "schema.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def old_writer_probe(baseline, out, gate, observations):
    probe = out.parent / "old-writer-probe"
    (probe / "src").mkdir(parents=True, exist_ok=True)
    dependency = json.dumps((baseline / LEDGER).as_posix(), ensure_ascii=False)
    (probe / "Cargo.toml").write_text(f"""[workspace]
[package]
name = "s1-v3-rollback-probe"
version = "0.1.0"
edition = "2024"
[dependencies]
agentguard-synthetic-ledger = {{ path = {dependency} }}
""", encoding="utf-8")
    (probe / "src/main.rs").write_text("""use agentguard_synthetic_ledger::{Error, Inspection, Ledger};
use std::{env, path::PathBuf};
fn main() {
    let paths: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    let v3 = Ledger::reopen(&paths[0], 7, 9).unwrap();
    drop(v3);
    Inspection::open(&paths[0]).unwrap().verify().unwrap();
    assert_eq!(Ledger::reopen(&paths[1], 7, 9).err(), Some(Error::Version));
    println!("G0-era code reopened independent v3 and rejected v4 writing");
}
""", encoding="utf-8")
    old_lock = (baseline / LEDGER / "Cargo.lock").read_text(encoding="utf-8")
    (probe / "Cargo.lock").write_text(old_lock, encoding="utf-8")
    gate("old-writer-lock", ["cargo", "metadata", "--offline", "--format-version", "1", "--manifest-path", str(probe / "Cargo.toml")], cwd=baseline / LEDGER)
    require(pins((probe / "Cargo.lock").read_text(encoding="utf-8")) == pins(old_lock), "rollback probe registry pins drifted")
    v3 = next(path.with_suffix(".sqlite3") for path, _ in observations if path.name == "F23-migration-rollback.json")
    v4 = next(path.with_suffix(".sqlite3") for path, _ in observations if path.name.startswith("F23-v3-independent-v4-copy-"))
    # The fixture directories and files belong to this invocation.
    with tempfile.TemporaryDirectory(prefix="agentguard-ledger-") as a, tempfile.TemporaryDirectory(prefix="agentguard-ledger-") as b:
        for root, source in ((Path(a), v3), (Path(b), v4)):
            (root / "fixture").write_bytes(b"AgentGuard I05 synthetic ledger\n")
            shutil.copyfile(source, root / "ledger.sqlite3")
        before = sha(Path(b) / "ledger.sqlite3")
        gate("old-writer-v3-v4", ["cargo", "run", "--locked", "--offline", "--manifest-path",
                                 str(probe / "Cargo.toml"), "--", a, b], cwd=baseline / LEDGER)
        require(sha(Path(b) / "ledger.sqlite3") == before, "old writer changed v4 bytes")


def pack(out, summary):
    (out / "summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    checks = "".join(f"{sha(path)}  {path.relative_to(out).as_posix()}\n" for path in files)
    checksum = out / "SHA256SUMS"
    checksum.write_text(checks, encoding="utf-8", newline="\n")
    archive = out.with_suffix(".zip")
    with ZipFile(archive, "w", compression=ZIP_DEFLATED) as evidence:
        for path in [*files, checksum]:
            evidence.write(path, path.relative_to(out).as_posix())
    archive.with_suffix(".zip.sha256").write_text(f"{sha(archive)}  {archive.name}\n", encoding="utf-8")
    print(f"Evidence {archive}", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-worktree", type=Path)
    args = parser.parse_args()
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    out = ROOT / ".work/i06-s/verified" / f"s1-{head[:12]}-{uuid.uuid4().hex[:8]}"
    out.mkdir(parents=True)
    env = dict(os.environ, AG_S1_PYTHON=sys.executable, AG_S1_OBSERVATIONS=str(out / "observations"), PYTHONUTF8="1")
    summary = {"result": "FAIL", "head": head, "i06_d_sha": D_SHA, "g0_sha": G0_SHA,
               "environment": {"os": platform.platform(), "python": sys.version.split()[0],
                               "github_run": env.get("GITHUB_RUN_ID"), "runner_image": env.get("ImageOS")},
               "gates": {}, "matrix": {f"F{i:02}": {"result": "NOT RUN"} for i in range(1,25)}}

    def gate(name, command, cwd=ROOT, timeout=840, process_env=env):
        log = out / f"{name}.log"
        result = subprocess.run(command, cwd=cwd, env=process_env, capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=timeout)
        log.write_text(result.stdout + result.stderr, encoding="utf-8")
        status = "PASS" if result.returncode == 0 else "FAIL"
        summary["gates"][name] = {"result": status, "command": command, "cwd": str(cwd),
                                  "sha": D_SHA if name.startswith("baseline-") or name.startswith("old-writer-") else head,
                                  "evidence": log.name}
        print(f"{status} {name}", flush=True)
        require(result.returncode == 0, f"{name} exited {result.returncode}. See {log}")
        return result.stdout

    try:
        require(os.name == "nt", "S1 gate requires Windows")
        require(not env.get("GITHUB_SHA") or env["GITHUB_SHA"] == head, "checkout and run SHA differ")
        gate("candidate-tracked-clean", ["git", "diff", "--exit-code", "HEAD"])
        require(not gate("candidate-untracked-clean", ["git", "status", "--porcelain"]).strip(), "candidate worktree is dirty")
        gate("frozen-paths", ["git", "diff", "--exit-code", D_SHA, "--", *FROZEN])
        gate("candidate-ancestry", ["git", "merge-base", "--is-ancestor", D_SHA, head])
        gate("accepted-g0-ledger", ["git", "diff", "--exit-code", G0_SHA, D_SHA, "--", LEDGER])
        summary["environment"]["rustc"] = gate("rust", ["rustc", "-vV"]).strip()
        summary["environment"]["node"] = gate("node", ["node", "--version"]).strip()
        original_ledger = subprocess.check_output(["git", "show", f"{D_SHA}:{LEDGER}/Cargo.lock"], cwd=ROOT, text=True)
        old_pins = pins(original_ledger)
        allowed = old_pins | pins((ROOT / TYPED / "Cargo.lock").read_text(encoding="utf-8")) | pins((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
        actual = pins((ROOT / LEDGER / "Cargo.lock").read_text(encoding="utf-8"))
        require(old_pins <= actual <= allowed, "ledger registry pins changed beyond already accepted versions")
        baseline = args.baseline_worktree
        if baseline is None:
            baseline = out.parent.parent / f"i06-d-baseline-{head[:12]}"
            if not baseline.exists():
                gate("baseline-checkout", ["git", "worktree", "add", "--detach", str(baseline), D_SHA])
        baseline = baseline.resolve()
        require(gate("baseline-head", ["git", "rev-parse", "HEAD"], cwd=baseline).strip() == D_SHA, "wrong baseline checkout")
        baseline_env = {name: value for name, value in env.items() if name not in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_SHA")}
        gate("baseline-i06-d", [sys.executable, "-B", "tools/verify_i06_d.py"], cwd=baseline, process_env=baseline_env)
        baseline_evidence = json.loads((baseline / ".work/i06-d/summary.json").read_text(encoding="utf-8"))
        require(baseline_evidence["head"] == D_SHA and baseline_evidence["result"] == "PASS", "baseline evidence mismatch")
        shutil.copytree(baseline / ".work/i06-d", out / "accepted-i06-d")
        gate("candidate-baseline", [sys.executable, "-B", "tools/baseline.py"])
        typed = ["--locked", "--manifest-path", f"{TYPED}/Cargo.toml", "--target-dir", ".work/i06-d-target"]
        gate("candidate-typed-format", ["cargo", "fmt", "--check", "--manifest-path", f"{TYPED}/Cargo.toml"])
        gate("candidate-typed-clippy", ["cargo", "clippy", *typed, "--all-targets", "--", "-D", "warnings"])
        typed_log = gate("candidate-typed-tests", ["cargo", "test", *typed])
        require(len(re.findall(r"(?m)^test tests::\w+ \.\.\. ok$", typed_log)) == 24, "I06-D test count mismatch")
        gate("candidate-six-domains", [sys.executable, "-B", "-m", "tools.check_i06_d_reference"])
        gate("candidate-frozen-vectors", [sys.executable, "-B", "-m", "tools.check_crypto"])

        ledger_log = gate("candidate-ledger", [sys.executable, "-B", "tools/verify_i05_ledger.py"])
        original = re.findall(r"(?m)^test tests::[\w:]+ \.\.\. ok$", ledger_log)
        require(len(original) == 27, "G0 ledger test count mismatch")
        shutil.copytree(ROOT / ".work/i05-ledger", out / "ledger-provenance")
        observed_paths = sorted((out / "observations").glob("F*.json"))
        observations = [(path, json.loads(path.read_text(encoding="utf-8"))) for path in observed_paths]
        report_schema(out, observations)
        old_writer_probe(baseline, out, gate, observations)
        for index in range(1, 25):
            label = f"F{index:02}"
            test = re.search(rf"(?m)^test store::s::tests::(f{index:02}_\w+) \.\.\. ok$", ledger_log)
            evidence = [path.relative_to(out).as_posix() for path in observed_paths if path.name.startswith(f"{label}-")]
            require(test and evidence, f"missing run or independent evidence for {label}")
            if index == 23:
                evidence.append("old-writer-v3-v4.log")
            summary["matrix"][label] = {"result": "PASS", "test": test[1], "sha": head,
                "command": summary["gates"]["candidate-ledger"]["command"],
                "environment": summary["environment"], "evidence": ["candidate-ledger.log", *evidence]}

        c2 = json.loads(gate("candidate-c2", [sys.executable, "-B", "probes/i05-c2/run.py"]))
        require(c2["result"] == "PASS" and c2["head"] == head, "C2 evidence mismatch")
        gate("candidate-first-batch", [sys.executable, "-B", "tools/ci.py"])
        first_batch = ROOT / ".work/ci"
        for name, expected in (("python-tests", "Ran 17 tests"), ("rust-tests", "14 passed; 0 failed"), ("windows-ipc", "Ran 13 tests")):
            require(expected in (first_batch / f"{name}.log").read_text(encoding="utf-8"), f"first-batch {name} mismatch")
        shutil.copytree(first_batch, out / "first-batch")
        summary.update(result="PASS", i06_d=24, g0_ledger=27, first_batch=44, c2="PASS",
                       s1_cases=24, read_only_snapshots=len(observations), frozen_design_files=42,
                       windows_ci="GitHub Actions job entry" if env.get("GITHUB_RUN_ID") else "local Windows job entry")
    finally:
        matrix = ["# S1 fault evidence", "", "| Case | Result | Test | Evidence |", "|---|---|---|---|"]
        for case, item in summary["matrix"].items():
            paths = " ".join(f"[{Path(path).name}]({path})" for path in item.get("evidence", []))
            matrix.append(f"| {case} | {item['result']} | {item.get('test','')} | {paths} |")
        (out / "matrix.md").write_text("\n".join(matrix) + "\n", encoding="utf-8")
        pack(out, summary)


if __name__ == "__main__":
    main()
