//! Isolated I06-D synthetic data model. Roles and observations are fixture inputs.
//! This crate has no OS, persistence, ingress or execution integration.
#![forbid(unsafe_code)]

mod clock;
mod crypto;
mod model;
mod snapshot;
mod state;

pub use clock::{Clock, ClockSample, Deadline, UtcMillis};
pub use crypto::{AuditLink, EventDigest, SyntheticKeys, redacted_event_digest};
pub use model::*;
pub use state::SyntheticCore;

#[cfg(test)]
mod tests;
