"""Verify the pinned SQLite build and run I05 ATs on Windows x64."""

import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile
import tomllib


REPO = Path(__file__).resolve().parents[1]
CRATE = REPO / "crates" / "ledger"
OUTPUT = REPO / ".work" / "i05-ledger"
TARGET = "x86_64-pc-windows-msvc"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
AMALGAMATION_SHA3 = "e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81"
SQLITE_BUILD_SCRIPT = "769b31370a41aa722e1d5a530012432c985bb142eab34d095481ffd1a3436e1b"


def check(ok, reason):
    if not ok:
        raise RuntimeError(reason)


def command(*args, timeout=30):
    process = subprocess.run(args, cwd=CRATE, capture_output=True, text=True,
                             encoding="utf-8", timeout=timeout)
    if process.returncode != 0:
        raise RuntimeError(f"{args[0]} failed ({process.returncode})\n"
                           f"{process.stdout}\n{process.stderr}")
    return process.stdout, process.stderr


def package_provenance():
    lock = tomllib.loads((CRATE / "Cargo.lock").read_text(encoding="utf-8"))
    entries = {(p["name"], p["version"]): p for p in lock["package"] if "source" in p}
    command("cargo", "fetch", "--locked", timeout=180)
    packages = json.loads(command("cargo", "metadata", "--format-version", "1",
                                  "--locked", "--offline")[0])["packages"]
    checked = []
    scripts = {}
    sqlite = None
    for package in packages:
        if package["source"] is None:
            continue
        name, version = package["name"], package["version"]
        entry = entries[(name, version)]
        check(package["source"] == entry["source"] == REGISTRY, f"unexpected registry for {name}")
        root = Path(package["manifest_path"]).parent
        archive = root.parents[2] / "cache" / root.parent.name / f"{name}-{version}.crate"
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        check(digest == entry["checksum"], f"bad crate archive checksum for {name}")
        checked.append({"name": name, "version": version, "crate_sha256": digest})
        for target in package["targets"]:
            if "custom-build" in target["kind"]:
                scripts[f"{name}@{version}"] = hashlib.sha256(
                    Path(target["src_path"]).read_bytes()
                ).hexdigest()
        if name == "libsqlite3-sys":
            check(version == "0.39.1", "unexpected SQLite bindings")
            sqlite = hashlib.sha3_256((root / "sqlite3" / "sqlite3.c").read_bytes()).hexdigest()
    check({(p["name"], p["version"]) for p in checked} == set(entries), "unverified lock entry")
    check(sqlite == AMALGAMATION_SHA3, "SQLite amalgamation differs from official 3.54.1")
    check(scripts.get("libsqlite3-sys@0.39.1") == SQLITE_BUILD_SCRIPT, "SQLite build script changed")
    return sorted(checked, key=lambda p: p["name"]), scripts


def main():
    check(os.name == "nt", "I05 gates require Windows")
    tmp = Path(tempfile.gettempdir()).resolve()
    check(ctypes.windll.kernel32.GetDriveTypeW(ctypes.c_wchar_p(tmp.anchor)) == 3,
          "temporary directory must use a local fixed disk")
    rust = command("rustc", "-vV")[0]
    check(rust.startswith("rustc 1.97.1 ") and f"host: {TARGET}\n" in rust,
          "unexpected Rust toolchain")
    check(command("cargo", "--version")[0].startswith("cargo 1.97.1 "), "unexpected Cargo")
    compiler_overrides = ("CFLAGS", "CC_", "LIBSQLITE3_", "SQLITE3_", "SQLITE_MAX_")
    check(not any(value for name, value in os.environ.items()
                  if name.upper().startswith(compiler_overrides)
                  or name.upper() in {"CL", "_CL_", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"}),
          "unexpected compiler or SQLite environment override")
    packages, scripts = package_provenance()
    command("cargo", "fmt", "--", "--check")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="agentguard-ledger-build-", dir=tmp) as target:
        common = ("--locked", "--offline", "--target", TARGET, "--target-dir", target)
        command("cargo", "clippy", *common, "--all-targets", "--", "-D", "warnings", timeout=180)
        stdout, stderr = command("cargo", "test", *common, "--", "--nocapture", timeout=180)
        build = Path(target) / TARGET / "debug" / "build"
        # Clippy and test may emit separate build-script directories. Every produced link must be static.
        link_files = list(build.glob("libsqlite3-sys-*/output"))
        check(bool(link_files) and all("cargo:rustc-link-lib=static=sqlite3" in
                                       path.read_text(encoding="utf-8") for path in link_files),
              "SQLite static linking was not verified")
    log = stdout + "\n" + stderr
    (OUTPUT / "at-results.log").write_text(log, encoding="utf-8")
    count = re.search(r"test result: ok\. (\d+) passed; 0 failed; 1 ignored", stdout)
    observations = [line for line in stdout.splitlines() if line.startswith("EVIDENCE ")]
    check(count is not None and int(count[1]) > 0, "AT test summary is missing")
    check(sum(" fault=" in line for line in observations) == 7, "missing fault evidence")
    check(any("omit=1 enable=0 load_function=missing" in line for line in observations),
          "extension loading must be compiled out and SQL function unavailable")
    for expected in ("AT21", "AT22/45", "AT29/45", "AT36", "AT45"):
        check(any(line.startswith(f"EVIDENCE {expected} ") for line in observations),
              f"missing evidence for {expected}")
    result = {
        "scope": "I05 synthetic ledger engineering gates",
        "head": command("git", "rev-parse", "HEAD")[0].strip(),
        "windows": platform.version(),
        "rustc": rust.splitlines()[0],
        "target": TARGET,
        "sqlite_amalgamation_sha3_256": AMALGAMATION_SHA3,
        "sqlite_link": "static=sqlite3",
        "packages": packages,
        "build_scripts_sha256": scripts,
        "passed": int(count[1]),
        "failed": 0,
        "driver_invoked_as_child": True,
        "observations": observations,
    }
    (OUTPUT / "evidence.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(stdout, end="")


if __name__ == "__main__":
    main()
