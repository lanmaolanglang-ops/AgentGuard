use crate::crypto::{digest, text};
use crate::model::require_ids;
use crate::{Failure, Observation, Preparation, Result, SyntheticKeys};
use agentguard_protocol::{crypto::Domain, parse::parse_json, schema::Schemas};
use serde_json::{Value, json};
use uuid::Uuid;

pub(crate) struct Snapshot {
    value: Value,
    pub(crate) invocation: Uuid,
    pub(crate) semantic: String,
    pub(crate) manifest: [u8; 32],
}

impl Snapshot {
    pub(crate) fn read(
        schemas: &Schemas,
        keys: &SyntheticKeys,
        observation: &Observation<'_>,
        os_boot_id: Uuid,
    ) -> Result<Self> {
        let Preparation {
            id,
            peer,
            generation,
            ..
        } = observation.preparation;
        require_ids(&[
            id,
            peer.connection,
            peer.broker_instance,
            peer.worker_instance,
        ])?;
        if generation == 0 || peer.broker_instance == peer.worker_instance {
            return Err(Failure::InvalidBinding);
        }
        observation.decision.eligible()?;
        let value = parse_json(observation.operation_json).map_err(|_| Failure::InvalidWire)?;
        schemas
            .validate("operation-v2", &value)
            .map_err(|_| Failure::InvalidWire)?;
        Self::validate_evidence(&value, os_boot_id)?;
        let invocation = parse_uuid(&value, "invocation_id")?;
        require_ids(&[invocation, parse_uuid(&value, "session_id")?])?;
        let payload = keys.semantic_payload(&value, observation.secret_kinds)?;
        Ok(Self {
            semantic: keys.sign(Domain::Semantic, &payload)?,
            manifest: digest(&value["handle_manifest"])?,
            invocation,
            value,
        })
    }

    pub(crate) fn validate_evidence(value: &Value, os_boot_id: Uuid) -> Result<()> {
        for key in ["requestor_principal", "executor_principal"] {
            let principal = &value[key];
            let process = &principal["process_instance"];
            let sandbox = &principal["sandbox_identity"];
            let boot_mismatch = parse_uuid(process, "os_boot_id")? != os_boot_id
                || (!principal["parent_process_instance"].is_null()
                    && parse_uuid(&principal["parent_process_instance"], "os_boot_id")?
                        != os_boot_id);
            if boot_mismatch
                || principal["integrity_level"] == "unknown"
                || matches!(sandbox["mechanism"].as_str(), Some("unknown" | "none"))
                || sandbox["identity_id"].is_null()
                || sandbox["policy_digest"].is_null()
                || principal["agent_runtime_identity"] != value["runtime"]
            {
                return Err(Failure::UnknownEvidence);
            }
        }
        // The state machine exercises one virtual read recipe. Cryptographic primitives
        // still support every frozen payload and all frozen reference vectors.
        if value["recipe"] != "file.read.v1" || value["working_directory"]["kind"] != "directory" {
            return Err(Failure::InvalidBinding);
        }
        let targets = value["targets"].as_array().ok_or(Failure::InvalidWire)?;
        let handles = value["handle_manifest"]
            .as_array()
            .ok_or(Failure::InvalidWire)?;
        let index = value["arguments"]["target_index"]
            .as_u64()
            .ok_or(Failure::InvalidWire)?;
        let [target] = targets.as_slice() else {
            return Err(Failure::InvalidBinding);
        };
        let [handle] = handles.as_slice() else {
            return Err(Failure::InvalidBinding);
        };
        const VIRTUAL_READ_ACCESS: u64 = 0x0012_0089;
        if index != 0
            || target["kind"] != "file"
            || target["content_digest"].is_null()
            || handle["object_type"] != "file"
            || handle["purpose"] != "target"
            || handle["inherit_to_worker"] != false
            || handle["granted_access"].as_u64() != Some(VIRTUAL_READ_ACCESS)
            || text(handle, "object_identity_digest")?
                != agentguard_protocol::crypto::hex(&digest(target)?)
        {
            return Err(Failure::InvalidBinding);
        }
        require_ids(&[parse_uuid(handle, "handle_ref")?])
    }

    pub(crate) fn matches(
        &self,
        keys: &SyntheticKeys,
        observation: &Observation<'_>,
    ) -> Result<()> {
        let current = parse_json(observation.operation_json).map_err(|_| Failure::InvalidWire)?;
        if current != self.value {
            return Err(Failure::InvalidBinding);
        }
        let payload = keys.semantic_payload(&current, observation.secret_kinds)?;
        keys.verify(Domain::Semantic, &payload, &self.semantic)
    }

    pub(crate) fn scope(&self) -> Value {
        let value = &self.value;
        json!({
            "allowed_agent": value["agent_id"], "runtime": value["runtime"],
            "provider": [value["provider_id"], value["provider_digest"]],
            "adapter": [value["adapter_id"], value["adapter_digest"]],
            "config": value["config_digest"],
            "principal": [value["requestor_principal"], value["executor_principal"]],
            "session": value["session_id"], "recipe": value["recipe"],
            "resource_scope": [value["working_directory"], value["targets"]],
        })
    }

    pub(crate) fn permit_identity(&self) -> Result<Value> {
        let value = &self.value;
        let hash = |key| digest(&value[key]).map(|bytes| agentguard_protocol::crypto::hex(&bytes));
        Ok(json!({
            "invocation_id": value["invocation_id"], "agent_id": value["agent_id"],
            "session_id": value["session_id"], "adapter_digest": value["adapter_digest"],
            "runtime_digest": hash("runtime")?,
            "requestor_principal_digest": hash("requestor_principal")?,
            "executor_principal_digest": hash("executor_principal")?,
        }))
    }
}

pub(crate) fn parse_uuid(value: &Value, key: &str) -> Result<Uuid> {
    Uuid::parse_str(text(value, key)?).map_err(|_| Failure::InvalidWire)
}
