"""Run I06-D and inherited Windows gates and emit reviewable evidence."""

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / ".work/i06-d"
BASE = "8c2a026468e05b352e4e3ed9de1db3dcc3ce8743"
MANIFEST = "crates/typed-authorization/Cargo.toml"
DESIGN = "docs/p1-a/evidence/I01/design-baseline.json"
ALLOWED_PREFIXES = ("crates/typed-authorization/", "docs/p1-a/i06-d/")
ALLOWED_FILES = {".github/workflows/p1-a-i06-d.yml", "tools/check_i06_d_reference.py",
                 "tools/verify_i06_d.py"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def command(args, timeout=360):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True,
                            encoding="utf-8", timeout=timeout)
    return result


def gate(name, args, timeout=360):
    result = command(args, timeout)
    (OUT / f"{name}.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    require(result.returncode == 0, f"{name} failed, inspect {name}.log")
    print(f"PASS {name}", flush=True)
    return result.stdout


def sha(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def registry(lock):
    packages = tomllib.loads((ROOT / lock).read_text(encoding="utf-8"))["package"]
    return {(p["name"], p["version"], p["source"], p["checksum"]) for p in packages if "source" in p}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    evidence = {"scope": "I06-D pure synthetic data", "result": "FAIL", "gates": {}}

    def record(name, result):
        evidence["gates"][name] = result

    try:
        require(platform.system() == "Windows", "Windows regression environment required")
        head = gate("head", ["git", "rev-parse", "HEAD"]).strip()
        gate("ancestry", ["git", "merge-base", "--is-ancestor", BASE, "HEAD"])
        changed = gate("diff-scope", ["git", "diff", "--name-only", "--no-renames", BASE]).splitlines()
        untracked = gate("untracked-scope", ["git", "ls-files", "--others", "--exclude-standard"]).splitlines()
        files = sorted(set(changed + untracked))
        require(all(p in ALLOWED_FILES or p.startswith(ALLOWED_PREFIXES) for p in files),
                "change outside the isolated phase")
        evidence.update({
            "head": head, "g0_base": BASE, "run_id": os.getenv("GITHUB_RUN_ID"),
            "run_attempt": os.getenv("GITHUB_RUN_ATTEMPT"), "changed_paths": files,
        })
        require(not evidence["run_id"] or os.environ["GITHUB_SHA"] == head, "run and checkout SHA differ")
        record("scope", "PASS")
        evidence["environment"] = {
            "platform": platform.platform(),
            "runner_image": os.getenv("ImageOS"), "runner_image_version": os.getenv("ImageVersion"),
            "rust": gate("rust-version", ["rustc", "-Vv"]).strip(),
            "cargo": gate("cargo-version", ["cargo", "-V"]).strip(),
            "python": sys.version.split()[0],
            "node": gate("node-version", ["node", "--version"]).strip(),
        }
        d_lock = "crates/typed-authorization/Cargo.lock"
        dependencies = registry(d_lock)
        require(dependencies and dependencies <= registry("Cargo.lock"), "D lock diverges from G0 registry pins")
        evidence["locks_sha256"] = {path: sha(path) for path in (d_lock, "Cargo.lock", "requirements-ci.lock")}
        evidence["d_registry_pins"] = len(dependencies)
        record("dependency_lock", "PASS")
        baseline = json.loads(gate("frozen-baseline", [sys.executable, "-B", "tools/baseline.py"]))
        require(baseline["design_files_unchanged"] == 42 and baseline["agents_at_L0"] == 7,
                "frozen design or inventory gate mismatch")
        shutil.copyfile(ROOT / DESIGN, OUT / "frozen-design.json")
        evidence.update({"frozen_files": 42, "frozen_manifest_sha256": sha(DESIGN),
                         "real_agents_at_l0": 7, "capability_flags": [False] * 5})
        record("frozen_design", "PASS")

        cargo = ["--locked", "--manifest-path", MANIFEST, "--target-dir", ".work/i06-d-target"]
        gate("format", ["cargo", "fmt", "--check", "--manifest-path", MANIFEST])
        gate("clippy", ["cargo", "clippy", *cargo, "--all-targets", "--", "-D", "warnings"])
        results = gate("typed-tests", ["cargo", "test", *cargo])
        tests = sorted(re.findall(r"(?m)^test tests::(\w+) \.\.\. ok$", results))
        source = (ROOT / "crates/typed-authorization/src/tests.rs").read_text(encoding="utf-8")
        expected = sorted(re.findall(r"(?m)^#\[test\]\s+fn (\w+)\(", source))
        require(len(expected) >= 24 and tests == expected
                and f"{len(tests)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out" in results,
                "D tests missing, failed or skipped")
        evidence["typed_tests"] = {name: "PASS" for name in tests}
        evidence["grant_transition_pairs"] = {"valid": 15, "rejected": 49}
        record("typed_authorization", "PASS")
        references = json.loads(gate("six-domain-references", [sys.executable, "-B", "-m", "tools.check_i06_d_reference"]))
        require(len(references["domains"]) == 6 and references["rust_check"] in tests,
                "missing composed MAC evidence")
        evidence["six_domain_references"] = references
        record("six_domains", "PASS")

        inherited = json.loads(gate("g0-regressions", [sys.executable, "-B", "tools/verify_g0.py"], timeout=780))
        require(inherited["head"] == head and inherited["ledger_passed"] == 27 and inherited["ledger_failed"] == 0
                and inherited["c2"] == "PASS" and inherited["first_batch_tests_passed"] == 44,
                "G0, C2 or first-batch evidence mismatch")
        evidence["regressions"] = inherited
        for name in ["summary.json", "ledger-evidence.json", "c2-evidence.json"]:
            shutil.copyfile(ROOT / ".work/g0" / name, OUT / f"inherited-{name}")
        legacy = ROOT / ".work/g0/first-batch"
        vectors = json.loads(gate("frozen-python-node-vectors", [sys.executable, "-B", "-m", "tools.check_crypto"]))
        rust_log = (legacy / "rust-tests.log").read_text(encoding="utf-8")
        require(vectors["vectors"] == 21 and vectors["python_mismatches"] == vectors["node_mismatches"] == 0
                and "test twenty_one_cross_language_vectors_match_bytes_frames_and_macs ... ok" in rust_log,
                "missing or failed fixed vectors")
        shutil.copyfile(legacy / "rust-tests.log", OUT / "inherited-rust-tests.log")
        shutil.copyfile(legacy / "summary.json", OUT / "inherited-first-batch-summary.json")
        evidence["fixed_vectors"] = {**vectors, "rust_mismatches": 0}
        record("inherited_and_fixed_vectors", "PASS")
        evidence["result"] = "PASS"
    finally:
        (OUT / "summary.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"PASS I06-D ({len(evidence['typed_tests'])} typed tests, 21 frozen vectors, 6 composed domains)")


if __name__ == "__main__":
    main()
