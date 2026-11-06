"""Independent construction of six public I06-D payloads with frozen encoders."""

import hashlib
import hmac
import json
import re
import struct
import subprocess

from tools.check_crypto import encode
from tools.extract_protocol import ROOT


def sha(value):
    return hashlib.sha256(encode(value)).hexdigest()


def mac(domain, payload, key_byte):
    domain = domain.encode("ascii")
    frame = b"AG2" + struct.pack(">H", len(domain)) + domain + encode(payload)
    return hmac.new(bytes([key_byte]) * 32, frame, hashlib.sha256).hexdigest()


def records():
    fixtures = json.loads((ROOT / "fixtures/p0.1.json").read_text(encoding="utf-8"))
    snapshot = next(row["value"] for row in fixtures if row["id"] == "OP01")
    snapshot["handle_manifest"][0]["object_identity_digest"] = sha(snapshot["targets"][0])
    invocation = snapshot.pop("invocation_id")
    key_id = "fixture-secret"
    secret = {"parameter_pointer": "/arguments/項~1目~0", "value_kind": "utf8", "value": "é🔒"}
    semantic_digest = mac("semantic-v2", snapshot, 2)
    epochs = {"core_boot_epoch": "00000000-0000-4000-8000-000000000602",
              "policy_epoch": 1, "grant_epoch": 2}
    intent = {"semantic_digest": semantic_digest, **epochs,
              "expires_at": "2026-11-05T08:01:00.000Z", "remaining_claims": 1}
    operation = {
        "semantic_digest": semantic_digest, "invocation_id": invocation,
        "grant_id": "00000000-0000-4000-8000-000000000600",
        "reservation_id": "00000000-0000-4000-8000-000000000601",
        "broker_instance": "00000000-0000-4000-8000-000000000603",
        "prepared_id": "00000000-0000-4000-8000-000000000604",
        "nonce": "bb" * 32, "key_id": key_id, **epochs,
        "expires_at": "2026-11-05T08:00:30.000Z", "ttl_ms": 30_000, "max_uses": 1,
    }
    permit = {
        **operation, "schema_version": "2.0.0",
        "operation_digest": mac("operation-v2", operation, 4),
        "agent_id": snapshot["agent_id"], "session_id": snapshot["session_id"],
        "runtime_digest": sha(snapshot["runtime"]), "adapter_digest": snapshot["adapter_digest"],
        "requestor_principal_digest": sha(snapshot["requestor_principal"]),
        "executor_principal_digest": sha(snapshot["executor_principal"]),
    }
    audit = {"segment_id": "00000000-0000-4000-8000-000000000998",
             "sequence": 1, "previous_mac": None, "event_digest": sha({"z": None, "a": 1}),
             "key_id": key_id}
    return [(f"{domain}-v2", payload, index) for index, (domain, payload) in enumerate((
        ("secret", secret), ("semantic", snapshot), ("intent", intent),
        ("operation", operation), ("permit", permit), ("audit", audit),
    ), start=1)]


def check():
    expected = json.loads((ROOT / "crates/typed-authorization/fixture-macs.json").read_text(encoding="utf-8"))
    rows = records()
    python = {domain: mac(domain, payload, index) for domain, payload, index in rows}
    if python != expected:
        raise ValueError("Python composed payload mismatch")
    document = (ROOT / "docs/p0.1/18-CRYPTOGRAPHIC-CONTRACT.md").read_text(encoding="utf-8")
    match = re.search(r"<!-- ag:crypto-node -->[\s\S]*?(function u32[\s\S]*?)\nconst key=", document)
    if match is None:
        raise ValueError("Frozen Node encoder missing")
    node = """import {createHmac} from 'node:crypto';
import {readFileSync} from 'node:fs';
""" + match.group(1) + """
const rows=JSON.parse(readFileSync(0,'utf8'));
const macs=Object.fromEntries(rows.map(([name,payload,keyByte])=>{
  const domain=Buffer.from(name,'ascii');
  const len=Buffer.alloc(2);
  len.writeUInt16BE(domain.length);
  const frame=Buffer.concat([Buffer.from('AG2'),len,domain,encode(payload)]);
  return [name,createHmac('sha256',Buffer.alloc(32,keyByte)).update(frame).digest('hex')];
}));
console.log(JSON.stringify(macs));
"""
    actual = subprocess.run(["node", "--input-type=module", "-e", node], input=json.dumps(rows),
                            capture_output=True, check=True, encoding="utf-8", timeout=30)
    if json.loads(actual.stdout) != expected:
        raise ValueError("Node composed payload mismatch")
    return {"domains": list(expected), "python_mismatches": 0, "node_mismatches": 0,
            "rust_check": "six_composed_macs_match_independent_references"}


if __name__ == "__main__":
    print(json.dumps(check()))
