//! AgentGuard I03 protocol primitives. No grant issuer, executor or Agent integration.
#![recursion_limit = "256"]
pub mod crypto;
pub mod ingress;
pub mod migration;
pub mod parse;
pub mod schema;

use serde::Serialize;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProtocolError {
    pub code: &'static str,
    pub message: &'static str,
}

impl ProtocolError {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for ProtocolError {}

pub type Result<T> = std::result::Result<T, ProtocolError>;
