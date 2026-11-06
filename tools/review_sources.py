"""Read public source metadata only, with bounded requests and no code execution."""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import urllib.request
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]


def entries():
    text = (ROOT / "docs/p0.1/13-SOURCE-REGISTER.md").read_text(encoding="utf-8")
    result = []
    for match in re.finditer(r"^### ([SN]\d{2}) (.*?)\n([\s\S]*?)(?=^### |^## |\Z)", text, re.M):
        identity, title, body = match.groups()
        links = re.findall(r"\]\((https://[^)]+)\)", body)
        if links:
            result.append((identity, title, links[-1]))
    if len(result) != 76:
        raise ValueError("来源条目数量不符")
    return result


def fetch(entry):
    identity, title, url = entry
    request = urllib.request.Request(url, headers={"User-Agent": "AgentGuard-I23-readonly-review/1.0"})
    result = {"id": identity, "title": title, "url": url, "operation": "public_get_only",
              "runtime_verified": False}
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            body = response.read(2 * 1024 * 1024)
            result.update(status=response.status, final_url=response.url,
                          sample_sha256=hashlib.sha256(body).hexdigest(),
                          sample_limited=len(body) == 2 * 1024 * 1024)
    except Exception as error:
        result.update(status=getattr(error, "code", None),
                      error_kind=type(error).__name__)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = list(pool.map(fetch, entries()))
    report = {"scope": "public_source_reachability_not_runtime_proof", "sources": results,
              "total": len(results), "http_200": sum(r.get("status") == 200 for r in results)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"total": report["total"], "http_200": report["http_200"]}))
