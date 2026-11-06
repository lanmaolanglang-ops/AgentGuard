"""Build and exercise the C2 probe using only disposable Windows test data."""

import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import tempfile
import time
import tomllib


ROOT = Path(__file__).resolve().parent
TARGET = "x86_64-pc-windows-msvc"
SOURCE_ID = "2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76"
AMALGAMATION_SHA3 = "e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81"
ROWS = ["1|committed|01", "2|committed|02", "rows=2", "integrity=ok"]


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def command(args, *, cwd=ROOT, timeout=20, expected=0):
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                            timeout=timeout, creationflags=subprocess.CREATE_NO_WINDOW)
    require(result.returncode == expected, f"command failed ({result.returncode}): {result.stderr}")
    return result.stdout


def fingerprint(path):
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def build(target_dir):
    rust = command(["rustc", "-vV"])
    require(rust.startswith("rustc 1.97.1 ") and f"host: {TARGET}\n" in rust,
            "C2 requires Rust 1.97.1 and Windows x64/MSVC")
    require(command(["cargo", "--version"]).startswith("cargo 1.97.1 "),
            "C2 requires Cargo 1.97.1")
    overrides = {
        "LIBSQLITE3_SYS_BUNDLING", "LIBSQLITE3_SYS_USE_PKG_CONFIG", "LIBSQLITE3_FLAGS",
        "SQLITE3_LIB_DIR", "SQLITE3_INCLUDE_DIR", "SQLITE3_STATIC",
        "SQLITE_MAX_VARIABLE_NUMBER", "SQLITE_MAX_EXPR_DEPTH", "SQLITE_MAX_COLUMN",
        "CL", "_CL_", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
    }
    require(not any(value for name, value in os.environ.items()
                    if name.upper() in overrides or name.upper().startswith(("CFLAGS", "CC_"))),
            "unexpected SQLite or compiler build override")

    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    locked = {(p["name"], p["version"]): p for p in lock["package"] if "source" in p}
    command(["cargo", "fetch", "--locked", "--target", TARGET], timeout=180)
    metadata = json.loads(command(["cargo", "metadata", "--locked", "--offline",
                                   "--format-version", "1", "--filter-platform", TARGET]))
    registry = "registry+https://github.com/rust-lang/crates.io-index"
    packages = []
    amalgamation = None
    build_script = None
    for package in metadata["packages"]:
        if package["source"] is None:
            continue
        name, version = package["name"], package["version"]
        entry = locked[(name, version)]
        require(package["source"] == entry["source"] == registry, "non-crates.io dependency")
        package_dir = Path(package["manifest_path"]).parent
        index = package_dir.parent.name
        archive = package_dir.parents[2] / "cache" / index / f"{name}-{version}.crate"
        checksum = fingerprint(archive)["sha256"]
        require(checksum == entry["checksum"], f"registry checksum mismatch for {name}")
        packages.append({"name": name, "version": version, "crate_sha256": checksum})
        if name == "libsqlite3-sys":
            require(version == "0.39.1", "unexpected libsqlite3-sys version")
            amalgamation = package_dir / "sqlite3" / "sqlite3.c"
            build_script = package_dir / "build.rs"
    require({(p["name"], p["version"]) for p in packages} == set(locked),
            "locked and resolved packages differ")
    require(amalgamation is not None and build_script is not None, "no bundled source found")
    source = amalgamation.read_bytes()
    require(hashlib.sha3_256(source).hexdigest() == AMALGAMATION_SHA3,
            "bundled SQLite differs from official 3.54.1 amalgamation")
    require(SOURCE_ID.encode("ascii") in source, "amalgamation source ID mismatch")
    build_script_sha256 = fingerprint(build_script)["sha256"]
    require(build_script_sha256 == "769b31370a41aa722e1d5a530012432c985bb142eab34d095481ffd1a3436e1b",
            "SQLite build script differs from reviewed source")
    command(["cargo", "build", "--offline", "--locked", "--target", TARGET, "--target-dir",
             str(target_dir)], timeout=240)
    out = target_dir / TARGET / "debug"
    outputs = list((out / "build").glob("libsqlite3-sys-*/output"))
    require(len(outputs) == 1 and "cargo:rustc-link-lib=static=sqlite3" in
            outputs[0].read_text(encoding="utf-8"), "static link proof is missing")
    binary = out / "agentguard-i05-c2.exe"
    return binary, {
        "rustc": rust.splitlines()[0],
        "target": TARGET,
        "packages": sorted(packages, key=lambda p: p["name"]),
        "sqlite_amalgamation_sha3_256": AMALGAMATION_SHA3,
        "sqlite_build_script_sha256": build_script_sha256,
        "link": "static=sqlite3",
        "binary": fingerprint(binary),
        "inputs": {path: fingerprint(ROOT / path) for path in
                   ("Cargo.toml", "Cargo.lock", "src/main.rs", "run.py")},
    }


def await_ready(child, marker):
    deadline = time.monotonic() + 15
    while not marker.is_file():
        if child.poll() is not None:
            raise RuntimeError(f"holder exited before fixture was ready: {child.stderr.read()}")
        require(time.monotonic() < deadline, "holder ready deadline expired")
        time.sleep(0.025)
    return dict(line.split("=", 1) for line in marker.read_text(encoding="utf-8").splitlines())


def wal_frames(path):
    data = path.read_bytes()
    magic, _, page_size = struct.unpack_from(">III", data)
    require(magic in (0x377F0682, 0x377F0683), "invalid WAL header")
    require(512 <= page_size <= 65536 and page_size & (page_size - 1) == 0,
            "invalid WAL page size")
    frame_size = 24 + page_size
    require((len(data) - 32) % frame_size == 0, "unexpected partial WAL frame")
    commits = [i for i, pos in enumerate(range(32, len(data), frame_size), 1)
               if struct.unpack_from(">I", data, pos + 4)[0] != 0]
    total = (len(data) - 32) // frame_size
    require(bool(commits) and total > commits[-1], "no uncommitted frames after last WAL commit")
    return {"page_size": page_size, "total_frames": total, "last_commit_frame": commits[-1]}


def exercise(binary, case):
    (case / "fixture").write_bytes(b"AgentGuard C2 synthetic fixture\n")
    with subprocess.Popen([binary, "hold"], cwd=case, stdout=subprocess.DEVNULL,
                          stderr=subprocess.PIPE, text=True, encoding="utf-8",
                          creationflags=subprocess.CREATE_NO_WINDOW) as child:
        try:
            ready = await_ready(child, case / "ready")
            require(ready["version"] == "3.54.1" and ready["source_id"] == SOURCE_ID,
                    "runtime SQLite identity differs")
            require([ready[k] for k in ("journal_mode", "synchronous", "foreign_keys",
                                        "wal_autocheckpoint")] == ["wal", "2", "1", "0"],
                    "unexpected runtime PRAGMA values")
            app_lock = command([binary, "contend"], cwd=case).strip()
            require(app_lock == "writer_lock=busy", "second process got app writer lock")
            start = time.monotonic()
            sqlite_busy = command([binary, "sqlite-busy"], cwd=case).strip()
            elapsed_ms = round((time.monotonic() - start) * 1000)
            require(sqlite_busy == "sqlite_busy=5" and elapsed_ms < 5000, "SQLite contention did not fail within budget")
            before = command([binary, "read"], cwd=case).splitlines()
            require(before == ROWS, "independent live reader saw unexpected rows")
            wal = case / "probe.sqlite3-wal"
            wal_before = fingerprint(wal)
            child.kill()
            child.wait(timeout=10)
            require(child.returncode != 0, "holder did not terminate abnormally")
            wal_after = fingerprint(wal)
            require(wal_after == wal_before, "crash changed WAL before independent reopen")
            require(wal_after["bytes"] == int(ready["uncommitted_wal"]) > int(ready["committed_wal"])
                    and ready["pending"] == "64",
                    f"WAL spill discrepancy: actual={wal_after['bytes']}, "
                    f"writer_committed={ready['committed_wal']}, writer_uncommitted={ready['uncommitted_wal']}, "
                    f"pending={ready['pending']}")
            frames = wal_frames(wal)
            database_after_kill = fingerprint(case / "probe.sqlite3")
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=10)

    reopened = command([binary, "reopen"], cwd=case).splitlines()
    independent = command([binary, "read"], cwd=case).splitlines()
    require(reopened == independent == ROWS, "recovery lost committed or restored uncommitted rows")
    full = command([binary, "full"], cwd=case).splitlines()
    after_full = command([binary, "read"], cwd=case).splitlines()
    require(full[0] == "sqlite_full=13" and after_full == ROWS,
            "SQLITE_FULL injection or rollback was not observed")
    return {
        "sqlite_runtime": ready,
        "app_writer_contention": app_lock,
        "sqlite_writer_contention": sqlite_busy,
        "sqlite_busy_child_wall_ms": elapsed_ms,
        "holder_pid": child.pid,
        "forced_exit_code": child.returncode,
        "wal_before_kill": wal_before,
        "wal_after_kill_before_reopen": wal_after,
        "wal_frames": frames,
        "database_after_kill_before_reopen": database_after_kill,
        "independent_read_while_writer_held": before,
        "new_writer_reopen": reopened,
        "independent_read_after_reopen": independent,
        "injected_page_limit_failure": full,
        "independent_read_after_full": after_full,
    }


def main():
    require(os.name == "nt", "C2 requires Windows")
    temp = Path(tempfile.gettempdir()).resolve()
    require(ctypes.windll.kernel32.GetDriveTypeW(ctypes.c_wchar_p(temp.anchor)) == 3,
            "C2 temp root must be on a local fixed drive")
    with tempfile.TemporaryDirectory(prefix="agentguard-c2-build-", dir=temp) as target:
        binary, provenance = build(Path(target))
        with tempfile.TemporaryDirectory(prefix="agentguard-c2-negative-", dir=temp) as outside:
            command([binary, "hold"], cwd=outside, expected=1)
            require(not any(Path(outside).iterdir()), "outside-fixture probe created files")
        with tempfile.TemporaryDirectory(prefix="agentguard-c2-", dir=temp) as case:
            result = exercise(binary, Path(case))
    print(json.dumps({
        "scope": "I05 C2 disposable SQLite only",
        "result": "PASS",
        "head": command(["git", "rev-parse", "HEAD"]).strip(),
        "windows": platform.version(),
        "outside_fixture_rejected": True,
        "build": provenance,
        "observations": result,
    }, indent=2))


if __name__ == "__main__":
    main()
