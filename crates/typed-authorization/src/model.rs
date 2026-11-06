use crate::{Deadline, UtcMillis};
use agentguard_protocol::parse::MAX_INTEGER;
use serde_json::Value;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    WrongRole,
    HardDeny,
    NativeDeny,
    UnknownEvidence,
    InvalidWire,
    InvalidBinding,
    InvalidState,
    Duplicate,
    UnknownRecord,
    RuleUnavailable,
    Exhausted,
    Expired,
    Epoch,
    Time,
    Key,
    Mac,
    NonceReuse,
    UnknownCommit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Core,
    Ui,
    Adapter,
    Broker,
    Worker,
    Observer,
}

pub(crate) fn require_role(actual: Role, expected: Role) -> Result<()> {
    (actual == expected).then_some(()).ok_or(Failure::WrongRole)
}

pub(crate) fn require_ids(ids: &[Uuid]) -> Result<()> {
    if ids.iter().any(Uuid::is_nil) {
        return Err(Failure::InvalidBinding);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyEpoch(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantEpoch(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CoreEpoch(pub Uuid);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Epochs {
    pub policy: PolicyEpoch,
    pub grant: GrantEpoch,
    pub core: CoreEpoch,
}

impl Epochs {
    pub(crate) fn validate(self) -> Result<()> {
        if self.policy.0 > MAX_INTEGER as u64
            || self.grant.0 > MAX_INTEGER as u64
            || self.core.0.is_nil()
        {
            return Err(Failure::Epoch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePermission {
    Allow,
    Deny,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyEvidence {
    Known,
    Unknown,
}

/// Trusted test inputs modeling a current Core decision and observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub id: Uuid,
    pub hard_deny: bool,
    pub native_permission: NativePermission,
    pub identity: SafetyEvidence,
    pub safety: SafetyEvidence,
}

impl Decision {
    pub(crate) fn eligible(self) -> Result<()> {
        if self.hard_deny {
            return Err(Failure::HardDeny);
        }
        match self.native_permission {
            NativePermission::Deny => return Err(Failure::NativeDeny),
            NativePermission::Unknown => return Err(Failure::UnknownEvidence),
            NativePermission::Allow => {}
        }
        if self.identity != SafetyEvidence::Known || self.safety != SafetyEvidence::Known {
            return Err(Failure::UnknownEvidence);
        }
        require_ids(&[self.id])
    }
}

/// Fixture instances and transport context. These identifiers do not authenticate a peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerBinding {
    pub connection: Uuid,
    pub broker_instance: Uuid,
    pub worker_instance: Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preparation {
    pub id: Uuid,
    pub peer: PeerBinding,
    pub generation: u64,
    pub deadline: Deadline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedStatus {
    Ready,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    Utf8,
    Base64Bytes,
}

/// Complete current input, supplied by the trusted synthetic caller at each boundary.
pub struct Observation<'a> {
    pub preparation: Preparation,
    pub decision: Decision,
    pub operation_json: &'a [u8],
    pub secret_kinds: &'a [SecretKind],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleRef {
    pub id: Uuid,
    pub version: u64,
}

/// The registered scope is derived from a known synthetic snapshot and cannot be widened.
pub struct RuleSpec {
    pub reference: RuleRef,
    pub purpose: String,
    pub valid_from: UtcMillis,
    pub expires_at: UtcMillis,
    pub max_operations: u32,
}

/// One synthetic UI approval of the rule and its exact prepared scope.
pub struct RuleIntent {
    pub(crate) spec: RuleSpec,
    pub(crate) prepared_id: Uuid,
    pub(crate) epochs: Epochs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManualApproval {
    pub approval_id: Uuid,
    pub grant_id: Uuid,
    pub parent: Option<RuleRef>,
    pub deadline: Deadline,
}

pub struct ReserveRequest {
    pub grant_id: Uuid,
    pub reservation_id: Uuid,
    pub nonce: [u8; 32],
    pub ttl_ms: u32,
    pub outcome: CommitObservation,
}

/// In-memory fault model. Lost confirmation models a change that committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitObservation {
    Confirmed,
    Rejected,
    ConfirmationLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantState {
    Created,
    Pending,
    Reserved,
    Consumed,
    Revoked,
    Expired,
    Invalidated,
    Failed,
}

impl GrantState {
    pub fn permits(self, next: Self) -> bool {
        use GrantState::*;
        matches!(
            (self, next),
            (Created, Pending) | (Pending, Reserved) | (Reserved, Consumed)
        ) || (self.is_open() && matches!(next, Revoked | Expired | Invalidated | Failed))
    }

    pub(crate) fn is_open(self) -> bool {
        matches!(self, Self::Created | Self::Pending | Self::Reserved)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    ManualRevoke,
    ParentRevoke,
    Deadline,
    BindingChanged,
    EpochChanged,
    KeyRotated,
    ClockUntrusted,
    ReserveRejected,
    ConsumeRejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantStatus {
    pub state: GrantState,
    pub reason: Option<EndReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconcileResult {
    UnknownRecord,
    Observed(GrantStatus),
    Unconfirmed(GrantStatus),
}

/// Separate types prevent a status or hypothetical operation result from becoming a ticket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulatedConsumption {
    Recorded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionResult {
    NotAttempted,
    ReportedSuccess,
    ReportedFailure,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionEvidence {
    None,
    UnverifiedReport,
}

/// One outbound synthetic response. Deliberately not Clone, Debug or Display.
pub struct Permit(pub(crate) Value);

impl Permit {
    pub fn into_wire(self) -> Vec<u8> {
        // Value is already validated, and writing to a Vec is infallible.
        serde_json::to_vec(&self.0).expect("JSON value serializes to Vec")
    }
}

pub enum ReserveResult {
    Issued(Permit),
    Rejected,
    Unknown,
}
