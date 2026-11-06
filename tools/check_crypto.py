"""Independent Python and frozen Node reference checks against all public vectors."""
import hashlib
import hmac
import json
import re
import struct
import subprocess
from tools.extract_protocol import ROOT


def encode(value):
    if value is None:
        return b"\x00"
    if value is False:
        return b"\x01"
    if value is True:
        return b"\x02"
    if isinstance(value, int):
        return b"\x03" + struct.pack(">q", value)
    if isinstance(value, str):
        raw = value.encode("utf-8", "strict")
        return b"\x04" + struct.pack(">I", len(raw)) + raw
    if isinstance(value, list):
        return b"\x06" + struct.pack(">I", len(value)) + b"".join(encode(v) for v in value)
    keys = sorted(value, key=lambda key: key.encode("utf-8"))
    return b"\x07" + struct.pack(">I", len(keys)) + b"".join(encode(k) + encode(value[k]) for k in keys)


def check():
    fixtures = json.loads((ROOT / "fixtures/p0.1.json").read_text(encoding="utf-8"))
    bundle = next(f["value"] for f in fixtures if f["id"] == "CV-BUNDLE")
    key = bytes.fromhex(bundle["key_hex"])
    for vector in bundle["vectors"]:
        encoded = encode(vector["input"])
        domain = vector["domain"].encode("ascii")
        framed = b"AG2" + struct.pack(">H", len(domain)) + domain + encoded
        assert encoded.hex() == vector["encoded_hex"], vector["id"]
        assert framed.hex() == vector["framed_hex"], vector["id"]
        assert hmac.new(key, framed, hashlib.sha256).hexdigest() == vector["hmac_sha256"], vector["id"]
    # This reviewed frozen build-time code has no input, file writes or network operations.
    document = (ROOT / "docs/p0.1/18-CRYPTOGRAPHIC-CONTRACT.md").read_text(encoding="utf-8")
    match = re.search(r"<!-- ag:crypto-node -->\s*\n~~~javascript\s*\n([\s\S]*?)\n~~~", document)
    if match is None:
        raise ValueError("Node参考实现缺失")
    node = json.loads(subprocess.check_output(["node", "--input-type=module", "-e", match.group(1)],
                                              encoding="utf-8"))
    for actual, expected in zip(node, bundle["vectors"], strict=True):
        for field in ("id", "encoded_hex", "framed_hex", "hmac_sha256"):
            assert actual[field] == expected[field], expected["id"]
    return {"vectors": len(bundle["vectors"]), "python_mismatches": 0, "node_mismatches": 0,
            "rust_vectors_checked_by": "cargo test contracts"}


if __name__ == "__main__":
    print(json.dumps(check()))
