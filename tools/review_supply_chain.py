"""Inventory locked development dependencies and query public OSV advisories."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT, text=True, encoding="utf-8"))
lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
locked = {(p["name"], p["version"]): p for p in lock["package"]}
packages = []
for package in metadata["packages"]:
    source = package["source"]
    if source is None:
        continue
    if source != "registry+https://github.com/rust-lang/crates.io-index":
        raise ValueError("出现未审查的依赖来源")
    pin = locked[(package["name"], package["version"])]
    if len(pin.get("checksum", "")) != 64:
        raise ValueError("缺失Cargo checksum")
    scripts = [t["src_path"] for t in package["targets"] if "custom-build" in t["kind"]]
    packages.append({
        "name": package["name"], "version": package["version"],
        "license": package["license"], "source": source, "checksum": pin["checksum"],
        "build_scripts": [{"file": Path(p).name,
                           "sha256": hashlib.sha256(Path(p).read_bytes()).hexdigest()} for p in scripts],
    })
queries = [{"package": {"name": p["name"], "ecosystem": "crates.io"}, "version": p["version"]}
           for p in packages]
for p in json.loads((ROOT / "docs/p1-a/evidence/I20-A/python-package-sources.json").read_text(encoding="utf-8")):
    queries.append({"package": {"name": p["name"], "ecosystem": "PyPI"}, "version": p["version"]})
advisories = {"service": "https://api.osv.dev/v1/querybatch", "status": "unavailable"}
try:
    request = urllib.request.Request(advisories["service"],
        data=json.dumps({"queries": queries}).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=45) as response:
        result = json.load(response)
    advisories = {"service": advisories["service"], "status": "checked",
                  "queries": len(queries),
                  "findings": [{"query": query, "vulns": row["vulns"]}
                               for query, row in zip(queries, result["results"]) if row.get("vulns")]}
except Exception as error:
    advisories["error_kind"] = type(error).__name__
output = ROOT / "docs/p1-a/evidence/I20-A"
output.mkdir(parents=True, exist_ok=True)
report = {"scope": "development_dependency_inventory_not_release_certification",
          "rust_packages": packages, "advisories": advisories,
          "sqlite_dependency": any(p["name"].lower().find("sqlite") >= 0 for p in packages)}
(output / "development-sbom.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"rust_registry_packages": len(packages), "build_script_packages": sum(bool(p["build_scripts"]) for p in packages),
                  "advisories": advisories, "sqlite_dependency": report["sqlite_dependency"]}))
