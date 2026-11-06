use crate::{ProtocolError, Result, crypto, parse::parse_json, schema::Schemas};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Adapter,
    Sensor,
    Ui,
    Broker,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    ReportEvent,
    Heartbeat,
    Evaluate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub major: u16,
    pub minor: u16,
    pub sequence: u64,
    pub challenge: String,
    pub method: Method,
    pub payload: Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    source_event_id: String,
    agent_id: String,
    tool_name: Option<String>,
    command: Option<String>,
    arguments: Value,
}

/// Connection-scoped synthetic validation only. This does not persist, authorize or execute.
pub struct Session {
    role: Role,
    challenge: String,
    sequence: u64,
    source_ids: HashMap<String, [u8; 32]>,
    schemas: Arc<Schemas>,
}

impl Session {
    pub fn new(role: Role, challenge: String, schemas: Arc<Schemas>) -> Self {
        Self {
            role,
            challenge,
            sequence: 0,
            source_ids: HashMap::new(),
            schemas,
        }
    }

    pub fn accept(&mut self, bytes: &[u8]) -> Result<Value> {
        let value = parse_json(bytes)?;
        let request: Request = serde_json::from_value(value).map_err(|_| {
            ProtocolError::new(
                "ingress_fields",
                "请求包含未开放的字段或方法，本次请求已拒绝。",
            )
        })?;
        if request.major != 2 || request.minor != 0 {
            return Err(ProtocolError::new(
                "protocol_version",
                "这个协议版本尚未支持，本次请求已拒绝。",
            ));
        }
        if request.challenge != self.challenge
            || request.sequence != self.sequence + 1
            || request.sequence > 16
        {
            return Err(ProtocolError::new(
                "connection_binding",
                "请求不属于当前连接或已过期，本次请求已拒绝。",
            ));
        }
        self.sequence = request.sequence;
        match request.method {
            Method::Heartbeat => {
                if request.payload != json!({}) {
                    return Err(ProtocolError::new(
                        "heartbeat_fields",
                        "状态检查包含不支持的字段。",
                    ));
                }
                Ok(
                    json!({"status":"synthetic_protocol_only","level":"L0","authorization_available":false}),
                )
            }
            Method::Evaluate => {
                if self.role != Role::Adapter {
                    return Err(ProtocolError::new(
                        "role_denied",
                        "当前组件没有申请此操作的权限。",
                    ));
                }
                Err(ProtocolError::new(
                    "authorization_unavailable",
                    "当前阶段仅校验合成协议，尚未提供授权执行，本次操作未获得许可。",
                ))
            }
            Method::ReportEvent => {
                if !matches!(self.role, Role::Adapter | Role::Sensor) {
                    return Err(ProtocolError::new(
                        "role_denied",
                        "当前组件没有提交行为提议的权限。",
                    ));
                }
                self.observation(request.payload)
            }
        }
    }

    fn observation(&mut self, payload: Value) -> Result<Value> {
        if ["tool_name", "command"]
            .iter()
            .any(|key| payload.get(key).is_none())
        {
            return Err(ProtocolError::new(
                "missing_observation_field",
                "行为字段必须显式提供，未知内容请使用null。",
            ));
        }
        let observation: Observation = serde_json::from_value(payload).map_err(|_| {
            ProtocolError::new(
                "protected_fields",
                "行为提议包含不支持的字段，审批、身份认证与能力结果由Core管理。",
            )
        })?;
        if !matches!(
            observation.agent_id.as_str(),
            "A01" | "A02" | "A03" | "A04" | "A05" | "A06" | "A07"
        ) || observation.source_event_id.is_empty()
            || observation.source_event_id.chars().count() > 128
            || observation
                .tool_name
                .as_ref()
                .is_some_and(|name| name.is_empty() || name.chars().count() > 128)
            || observation
                .command
                .as_ref()
                .is_some_and(|command| command.is_empty() || command.chars().count() > 32760)
        {
            return Err(ProtocolError::new(
                "observation_bounds",
                "行为提议的身份或内容长度无效。",
            ));
        }
        self.schemas.validate_arguments(&observation.arguments)?;
        let digest: [u8; 32] = Sha256::digest(crypto::typed_encode(
            &serde_json::to_value(&observation).unwrap(),
        )?)
        .into();
        let duplicate = match self.source_ids.get(&observation.source_event_id) {
            Some(previous) if previous != &digest => {
                return Err(ProtocolError::new(
                    "source_conflict",
                    "同一事件编号出现不同内容，已停止接收这项提议。",
                ));
            }
            Some(_) => true,
            None => false,
        };
        self.source_ids.insert(observation.source_event_id, digest);
        Ok(json!({
            "status": if duplicate {"duplicate"} else {"validated_not_persisted"},
            "record_kind":"synthetic_fixture", "executed":false, "authorization_available":false
        }))
    }
}
