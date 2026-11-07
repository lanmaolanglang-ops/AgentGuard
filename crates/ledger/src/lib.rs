//! I05 local synthetic ledger. Mutations produce persistence receipts only.
#![forbid(unsafe_code)]

mod db;
mod store;
#[cfg(test)]
mod tests;
mod writer;

pub use db::{Archive, Inspection};
pub use store::Ledger;
pub use store::s;
pub use writer::{Client, Writer};

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use sha2::{Digest as _, Sha256};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Bytes<const N: usize>(pub [u8; N]);

pub type Id = Bytes<16>;
pub type Digest = Bytes<32>;

impl<const N: usize> ToSql for Bytes<N> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Blob(&self.0)))
    }
}

impl<const N: usize> FromSql for Bytes<N> {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let blob = value.as_blob()?;
        Ok(Self(blob.try_into().map_err(|_| {
            FromSqlError::InvalidBlobSize {
                expected_size: N,
                blob_size: blob.len(),
            }
        })?))
    }
}

pub fn nonce_hash(raw_nonce: &[u8; 32]) -> Digest {
    Bytes(Sha256::digest(raw_nonce).into())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Epoch {
    pub boot: Id,
    pub policy: u32,
    pub grant: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding {
    pub grant: Id,
    pub decision: Id,
    pub input: Digest,
    pub epoch: Epoch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Created,
    Pending,
    Reserved,
    Consumed,
    Revoked,
    Expired,
    Invalidated,
    Failed,
}

impl State {
    pub const ALL: [Self; 8] = [
        Self::Created,
        Self::Pending,
        Self::Reserved,
        Self::Consumed,
        Self::Revoked,
        Self::Expired,
        Self::Invalidated,
        Self::Failed,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Pending => "pending",
            Self::Reserved => "reserved",
            Self::Consumed => "consumed",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::Invalidated => "invalidated",
            Self::Failed => "failed",
        }
    }

    fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Pending)
                | (Self::Pending, Self::Reserved)
                | (Self::Reserved, Self::Consumed)
                | (
                    Self::Created | Self::Pending | Self::Reserved,
                    Self::Revoked | Self::Expired | Self::Invalidated | Self::Failed
                )
        )
    }
}

impl FromSql for State {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let name = value.as_str()?;
        Self::ALL
            .into_iter()
            .find(|s| s.name() == name)
            .ok_or(FromSqlError::InvalidType)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Create { eligible: bool },
    Pend,
    Reserve { nonce: Digest, ttl_ms: u32 },
    Consume { nonce: Digest },
    Revoke,
    Expire,
    Invalidate,
    FailPreparation,
    WorkerFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mutation {
    pub request: Id,
    pub binding: Binding,
    pub action: Action,
}

impl Mutation {
    fn fingerprint(self) -> Digest {
        let mut hash = Sha256::new();
        hash.update(b"AgentGuard/I05/local-request/v1\0");
        hash.update(self.binding.grant.0);
        hash.update(self.binding.decision.0);
        hash.update(self.binding.input.0);
        hash.update(self.binding.epoch.boot.0);
        hash.update(self.binding.epoch.policy.to_be_bytes());
        hash.update(self.binding.epoch.grant.to_be_bytes());
        match self.action {
            Action::Create { eligible } => hash.update([0, u8::from(eligible)]),
            Action::Pend => hash.update([1]),
            Action::Reserve { nonce, ttl_ms } => {
                hash.update([2]);
                hash.update(nonce.0);
                hash.update(ttl_ms.to_be_bytes());
            }
            Action::Consume { nonce } => {
                hash.update([3]);
                hash.update(nonce.0);
            }
            Action::Revoke => hash.update([4]),
            Action::Invalidate => hash.update([5]),
            Action::FailPreparation => hash.update([6]),
            Action::WorkerFailed => hash.update([7]),
            Action::Expire => hash.update([8]),
        }
        Bytes(hash.finalize().into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub sequence: i64,
    pub grant: Id,
    pub state: State,
    pub kind: AuditKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditKind {
    Transition,
    CancelRequested,
    WorkerFailed,
}

impl AuditKind {
    fn name(self) -> &'static str {
        match self {
            Self::Transition => "transition",
            Self::CancelRequested => "cancel_requested",
            Self::WorkerFailed => "worker_failed",
        }
    }
}

impl FromSql for AuditKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "transition" => Ok(Self::Transition),
            "cancel_requested" => Ok(Self::CancelRequested),
            "worker_failed" => Ok(Self::WorkerFailed),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Durable(Receipt),
    Reconciled(Receipt),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    pub source: Id,
    pub event: Id,
    pub sequence: i64,
    pub digest: Digest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ingest {
    Durable { sequence: i64 },
    Duplicate { sequence: i64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Scope,
    Storage,
    Build,
    Version,
    Corrupt,
    Archived,
    Busy,
    QueueFull,
    Fenced,
    UnknownCommit,
    Clock,
    Conflict,
    Epoch,
    Binding,
    Ineligible,
    Transition,
    Replay,
    Input,
    Sqlite(i32),
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        match error {
            rusqlite::Error::SqliteFailure(code, _) => match code.code {
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                    Self::Busy
                }
                rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase => {
                    Self::Corrupt
                }
                _ => Self::Sqlite(code.extended_code),
            },
            _ => Self::Corrupt,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
