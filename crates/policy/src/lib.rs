//! Deterministic classification for checked-in synthetic fixtures only.
#![forbid(unsafe_code)]

mod fixtures;
#[cfg(test)]
mod tests;

pub use fixtures::FixtureId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Risk {
    Low,
    High,
    Critical,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    Deny,
    Review,
    SyntheticCandidate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    ProtectedTarget,
    NativeDenied,
    NativePermissionUnknown,
    MissingEvidence,
    OutsideScope,
    DestructiveWithoutGrant,
    OpaqueEffects,
    LifecycleMayRun,
    ScopedRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    ReadOnly,
    Destructive,
    LifecycleMayRun,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceGap {
    Identity,
    Object,
    NativePermission,
    Effects,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    pub risk: Risk,
    pub disposition: Disposition,
    pub reason: Reason,
    pub effect: Effect,
    pub target: Option<&'static str>,
    pub reversible: Option<bool>,
    pub gap: Option<EvidenceGap>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Decision {
    fixture: FixtureId,
    proposal: Proposal,
    classification: Classification,
}

impl Decision {
    pub fn classification(&self) -> &Classification {
        &self.classification
    }

    pub fn fixture(&self) -> FixtureId {
        self.fixture
    }

    pub fn policy_epoch(&self) -> u64 {
        self.proposal.policy_epoch
    }

    pub fn matches_fixture(&self, fixture: FixtureId) -> bool {
        self.fixture == fixture && self.matches_proposal(&fixtures::load(fixture).proposal)
    }

    fn matches_proposal(&self, proposal: &Proposal) -> bool {
        &self.proposal == proposal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    FileRead,
    BatchDelete,
    OpaqueShell,
    PythonSubprocess,
    NpmLifecycle,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scope {
    InFixtureRoot,
    OutsideFixtureRoot,
    Protected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Native {
    FixtureAllowsReview,
    Denied,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Proposal {
    action: Action,
    scope: Scope,
    target: Option<&'static str>,
    object: Option<&'static str>,
    native: Native,
    command: Option<&'static str>,
    arguments: &'static [&'static str],
    cwd: &'static str,
    requestor: Option<&'static str>,
    executor: Option<&'static str>,
    runtime: &'static str,
    provider: &'static str,
    config: &'static str,
    session: &'static str,
    invocation: &'static str,
    executable: &'static str,
    script: Option<&'static str>,
    environment: &'static str,
    stdin: Option<&'static str>,
    network: Option<&'static str>,
    policy_epoch: u64,
    grant_epoch: u64,
    core_epoch: &'static str,
}

/// Only compiled-in fixture IDs can enter the classifier.
pub fn classify(fixture: FixtureId) -> Decision {
    let proposal = fixtures::load(fixture).proposal;
    Decision {
        fixture,
        proposal,
        classification: classify_proposal(&proposal),
    }
}

fn classify_proposal(proposal: &Proposal) -> Classification {
    use Disposition::{Deny, Review, SyntheticCandidate};
    let effect = match proposal.action {
        Action::FileRead
            if proposal.command.is_none()
                && proposal.script.is_none()
                && proposal.stdin.is_none()
                && proposal.network.is_none() =>
        {
            Effect::ReadOnly
        }
        Action::BatchDelete => Effect::Destructive,
        Action::NpmLifecycle => Effect::LifecycleMayRun,
        _ => Effect::Unknown,
    };
    let (risk, disposition, reason, gap) = if proposal.scope == Scope::Protected {
        (Risk::Critical, Deny, Reason::ProtectedTarget, None)
    } else if proposal.native == Native::Denied {
        (Risk::Unknown, Deny, Reason::NativeDenied, None)
    } else if proposal.requestor.is_none() || proposal.executor.is_none() {
        (
            Risk::Unknown,
            Review,
            Reason::MissingEvidence,
            Some(EvidenceGap::Identity),
        )
    } else if proposal.native == Native::Unknown {
        (
            Risk::Unknown,
            Review,
            Reason::NativePermissionUnknown,
            Some(EvidenceGap::NativePermission),
        )
    } else {
        match proposal.action {
            Action::BatchDelete => (Risk::High, Deny, Reason::DestructiveWithoutGrant, None),
            Action::NpmLifecycle => (
                Risk::High,
                Review,
                Reason::LifecycleMayRun,
                Some(EvidenceGap::Effects),
            ),
            Action::FileRead if proposal.scope == Scope::OutsideFixtureRoot => {
                (Risk::High, Deny, Reason::OutsideScope, None)
            }
            Action::FileRead
                if proposal.scope != Scope::InFixtureRoot
                    || proposal.target.is_none()
                    || proposal.object.is_none() =>
            {
                (
                    Risk::Unknown,
                    Review,
                    Reason::MissingEvidence,
                    Some(EvidenceGap::Object),
                )
            }
            Action::FileRead if effect == Effect::ReadOnly => {
                (Risk::Low, SyntheticCandidate, Reason::ScopedRead, None)
            }
            _ => (
                Risk::Unknown,
                Review,
                Reason::OpaqueEffects,
                Some(EvidenceGap::Effects),
            ),
        }
    };
    Classification {
        risk,
        disposition,
        reason,
        effect,
        target: proposal.target,
        reversible: (effect == Effect::Destructive).then_some(false),
        gap,
    }
}
