"""Independent SQLite read-only evidence, with no authority or signing dependency."""
import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import uuid


def inspect(root, fence=None, snapshot=None):
    path = Path(root) / "ledger.sqlite3"
    with sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True) as db:
        db.execute("PRAGMA query_only=ON")
        db.execute("BEGIN")
        result = {
            "reader": "python sqlite3, URI mode=ro, query_only, single snapshot",
            "sqlite_version": sqlite3.sqlite_version,
            "version": db.execute("PRAGMA user_version").fetchone()[0],
            "integrity": db.execute("PRAGMA integrity_check").fetchone()[0],
            "foreign_key_faults": len(db.execute("PRAGMA foreign_key_check").fetchall()),
        }
        assert result["integrity"] == "ok" and result["foreign_key_faults"] == 0
        result["states"] = dict(db.execute("SELECT state,count(*) FROM grants GROUP BY state"))
        tables = ("s_approval_claims", "s_reservations", "s_launch_once", "s_requests", "s_audit",
                  "s_execution_facts", "nonces", "execution", "g0_reservations")
        if result["version"] == 4:
            activation = db.execute("SELECT a.seq,hex(a.lineage),hex(a.core),hex(a.clock_domain),hex(a.os_boot),a.generation FROM s_activation_history a JOIN s_meta m ON m.activation=a.seq").fetchone()
            result["fence"] = {"activation": activation[0], "generation": activation[5], "status": "unavailable"}
            if fence:
                marker = Path(fence) / "s-fence"
                if marker.exists():
                    expected = "AgentGuard S1 synthetic fence\n" + "\n".join([
                        str(uuid.UUID(activation[1])), str(activation[0]), str(uuid.UUID(activation[2])),
                        str(uuid.UUID(activation[3])), str(uuid.UUID(activation[4])), str(activation[5])]) + "\n"
                    result["fence"]["status"] = "matched" if marker.read_bytes() == expected.encode() else "mismatch"
                    result["fence"]["sha256"] = hashlib.sha256(marker.read_bytes()).hexdigest()
            result["counts"] = {name: db.execute(f"SELECT count(*) FROM {name}").fetchone()[0] for name in tables}
            result["binding_faults"] = db.execute("SELECT count(*) FROM s_binding_faults").fetchone()[0]
            assert result["binding_faults"] == 0
            # Independently express cardinality and quota equations, without the S view.
            result["used"] = [row[0] for row in db.execute("SELECT used FROM s_rules ORDER BY id")]
            claims = db.execute("SELECT count(*) FROM s_approval_claims c JOIN s_approval_versions a ON a.id=c.approval AND a.version=c.version WHERE a.rule_id IS NOT NULL").fetchone()[0]
            assert sum(result["used"]) == claims
            assert result["counts"]["s_approval_claims"] == result["counts"]["s_reservations"]
            assert result["states"].get("consumed", 0) == result["counts"]["s_launch_once"]
            assert db.execute("SELECT count(*) FROM s_reservations s JOIN g0_reservations g ON s.grant_id=g.grant_id WHERE s.wall!=g.wall_deadline OR s.mono!=g.mono_deadline OR s.nonce_hash!=g.nonce_hash").fetchone()[0] == 0
            assert db.execute("SELECT count(*) FROM s_requests r JOIN s_audit a ON a.seq=r.audit_seq WHERE a.request!=r.id").fetchone()[0] == 0
            result["activation"] = db.execute("SELECT activation FROM s_meta").fetchone()[0]
            result["rule_versions"] = db.execute("SELECT count(*) FROM s_rule_versions").fetchone()[0]
            result["execution_states"] = dict(db.execute("SELECT state,count(*) FROM execution GROUP BY state"))
        objects = db.execute("SELECT type,name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY type,name").fetchall()
        normalized = [(kind, name, sql.replace("\r\n", "\n")) for kind, name, sql in objects]
        result["schema_sha256"] = hashlib.sha256(json.dumps(normalized, ensure_ascii=False).encode()).hexdigest()
        if snapshot:
            snapshot = Path(snapshot)
            snapshot.parent.mkdir(parents=True, exist_ok=True)
            with closing(sqlite3.connect(snapshot)) as destination:
                db.backup(destination)
            result["snapshot_sha256"] = hashlib.sha256(snapshot.read_bytes()).hexdigest()
        db.rollback()
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("fixture")
    parser.add_argument("--output")
    parser.add_argument("--fence")
    args = parser.parse_args()
    snapshot = Path(args.output).with_suffix(".sqlite3") if args.output else None
    result = inspect(args.fixture, args.fence, snapshot)
    # Only hashes and aggregate facts are exported. Raw nonce and responses are never selected.
    result["files_sha256"] = {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in Path(args.fixture).glob("ledger.sqlite3*") if p.is_file()
    }
    encoded = json.dumps(result, indent=2)
    if args.output:
        Path(args.output).parent.mkdir(parents=True, exist_ok=True)
        Path(args.output).write_text(encoded + "\n", encoding="utf-8")
    print(encoded)
