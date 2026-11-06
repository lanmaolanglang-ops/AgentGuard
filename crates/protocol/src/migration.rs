use crate::{ProtocolError, Result, parse::parse_json, schema::Schemas};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ImportedEvent {
    pub event: Value,
    pub original_event_id: String,
    pub original_sha256: String,
}

fn mapped_id(value: &Value) -> Value {
    match value.as_str() {
        Some(text) => json!(
            Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("agentguard-import:{text}").as_bytes()
            )
            .to_string()
        ),
        None => Value::Null,
    }
}

pub fn empty_capabilities(scope_id: &str) -> Value {
    json!({
        "scope_id": scope_id, "level": "L0", "telemetry_verified": false,
        "pre_execution_block_verified": false, "parameter_binding_verified": false,
        "single_use_grant_verified": false, "os_isolation_verified": false,
        "certificate": null
    })
}

/// Migrate historical observations only. No previous approval or identity becomes authoritative.
pub fn import_legacy(
    schemas: &Schemas,
    raw: &[u8],
    received_at: &str,
    import_instance: &str,
    ingest_seq: u64,
) -> Result<ImportedEvent> {
    let old = parse_json(raw)?;
    schemas.validate("event-v1", &old)?;
    let event_id = old["event_id"].as_str().unwrap();
    let evidence: Vec<_> = old["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, item)| {
            json!({
                "evidence_id": Uuid::new_v5(&Uuid::NAMESPACE_URL,
                    format!("{event_id}{index}").as_bytes()).to_string(),
                "grade": "E0", "kind": "claimed", "description": item["summary"],
                "artifact_digest": null
            })
        })
        .collect();
    let targets: Vec<_> = old["target_paths"].as_array().unwrap().iter().map(|target| {
        json!({"path": target["value"], "verification": "claimed", "object_digest": null})
    }).collect();
    let network: Vec<_> = old["network_destinations"].as_array().unwrap().iter().map(|item| {
        let scheme = match item["protocol"].as_str() {
            Some("http") => "http",
            Some("https") => "https",
            _ => "other",
        };
        json!({
            "authorization_id": null, "scope_id": null, "request_method": null,
            "scheme": scheme, "host": item["host"], "port": item["port"], "destination_ip": null,
            "dns_answers": [], "dns_epoch": null, "dns_expires_at": null, "resolver_id": null,
            "redirect_chain": [], "redirect_limit": 0,
            "proxy": {"mode": "unknown", "proxy_id": null, "endpoint_digest": null},
            "transport": "unknown", "path_query_digest": null, "upload_data_digest": null,
            "request_body_digest": null, "max_request_bytes": null, "max_response_bytes": null,
            "max_requests": null, "connection_id": null, "pool_scope": null,
            "connection_policy_epoch": null, "observation_state": "claimed",
            "enforcement_state": "none", "enforcement_scope_id": null, "native_permission": "unknown"
        })
    }).collect();
    let retained_states = [
        "proposed",
        "pending",
        "denied",
        "running",
        "succeeded",
        "failed",
        "cancelled",
        "unknown",
    ];
    let execution_state = old["execution_state"]
        .as_str()
        .filter(|state| retained_states.contains(state))
        .unwrap_or("unknown");
    let principal = if old["process_id"].is_null() {
        json!({"status": "unknown", "reported": null, "os_verified": null,
               "verifier_instance": null, "verified_at": null})
    } else {
        json!({"status": "claimed", "reported": {
            "product": old["agent_id"], "version": old["agent_version"],
            "pid": old["process_id"], "sid": null
        }, "os_verified": null, "verifier_instance": null, "verified_at": null})
    };
    let event = json!({
        "schema_version": "2.0.0", "record_kind": "imported", "event_id": old["event_id"],
        "agent_id": old["agent_id"], "agent_version": old["agent_version"],
        "session_id": mapped_id(&old["session_id"]),
        "parent_session_id": mapped_id(&old["parent_session_id"]),
        "subagent_id": old.get("subagent_id").cloned().unwrap_or(Value::Null),
        "parent_invocation_id": null, "depth": 0, "provider_id": null,
        "event_type": old["event_type"], "timestamp": old["timestamp"],
        "received_at": received_at, "correlation_id": mapped_id(&old["correlation_id"]),
        "ingest_seq": ingest_seq, "tool_name": old["tool_name"], "command": old["command"],
        "arguments": old["arguments"], "working_directory": old["working_directory"],
        "target_paths": targets, "network_requests": network, "principal_snapshot": principal,
        "execution_mode": "none", "execution_state": execution_state,
        "policy_decision": "not_evaluated", "risk_level": "unknown",
        "capabilities": empty_capabilities("00000000-0000-4000-8000-000000000900"),
        "grant": null, "broker_instance": null, "prepared_id": null,
        "semantic_digest": null, "operation_digest": null, "decision_id": null,
        "source": {"kind": "import", "instance_id": import_instance,
            "sequence": ingest_seq, "invocation_id": null, "authenticated": false,
            "source_event_id": format!("legacy-{event_id}")},
        "evidence": evidence, "missing_fields": [
            "principal_snapshot.os_verified", "migration.runtime_certification"],
        "redaction": {"profile": "audit-safe-v2", "fields": [], "raw_retained": false},
        "extensions": {}
    });
    schemas.validate("event-v2", &event).map_err(|_| {
        ProtocolError::new(
            "migration_bounds",
            "历史记录无法完整表示为当前协议，已停止导入，请保留原始记录。",
        )
    })?;
    Ok(ImportedEvent {
        event,
        original_event_id: event_id.to_owned(),
        original_sha256: crate::crypto::hex(&Sha256::digest(raw)),
    })
}
