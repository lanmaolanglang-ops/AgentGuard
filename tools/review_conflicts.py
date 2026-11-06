"""Re-read fixed public source conflicts without executing repository code."""
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
COMMIT = "f4f32cc7d3078f7031c7852fbf0b7de410958199"
results = []
for name in ("GUIDE.md", "CONFIG_PATHS.md", "EXTENSION_PROTOCOL.md"):
    endpoint = f"repos/esengine/DeepSeek-Reasonix/contents/docs/{name}?ref={COMMIT}"
    raw = json.loads(subprocess.check_output(["gh", "api", endpoint], encoding="utf-8"))
    text = base64.b64decode(raw["content"]).decode("utf-8")
    matched = [{"line": i + 1, "text": line[:600]} for i, line in enumerate(text.splitlines())
               if re.search(r"Windows|full.trust|host.*deny|PermissionRequest|PreToolUse|spawn|Protocol v|timeout", line, re.I)]
    results.append({"source": raw["html_url"], "commit": COMMIT,
                    "sha256": hashlib.sha256(text.encode()).hexdigest(),
                    "matched_lines": matched[:60], "runtime_verified": False})
path = ROOT / "docs/p1-a/evidence/I23/fixed-source-excerpts.json"
path.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps(results, ensure_ascii=False))
