use super::{FixtureId::*, *};

fn expect(fixture: FixtureId, risk: Risk, disposition: Disposition, reason: Reason) -> Decision {
    let decision = classify(fixture);
    assert_eq!(decision.fixture(), fixture);
    let result = decision.classification();
    assert_eq!(
        (result.risk, result.disposition, result.reason),
        (risk, disposition, reason)
    );
    decision
}

#[test]
fn at01_scoped_read_requires_fixture_identity_object_and_scope() {
    let decision = expect(
        ScopedRead,
        Risk::Low,
        Disposition::SyntheticCandidate,
        Reason::ScopedRead,
    );
    assert_eq!(decision.classification().effect, Effect::ReadOnly);
    assert_eq!(
        decision.classification().target,
        Some("fixture://workspace/source.rs")
    );
    assert_eq!(decision.classification().gap, None);
    assert_eq!(decision.policy_epoch(), 1);
    assert!(decision.matches_fixture(ScopedRead));
    for (fixture, gap) in [
        (MissingRequestor, EvidenceGap::Identity),
        (MissingObject, EvidenceGap::Object),
    ] {
        let unknown = expect(
            fixture,
            Risk::Unknown,
            Disposition::Review,
            Reason::MissingEvidence,
        );
        assert_eq!(unknown.classification().gap, Some(gap));
        assert!(!decision.matches_fixture(fixture));
    }
    expect(
        OutsideRead,
        Risk::High,
        Disposition::Deny,
        Reason::OutsideScope,
    );
}

#[test]
fn at02_destruction_and_hard_deny_cannot_be_approved_by_ui() {
    let decision = expect(
        BatchDelete,
        Risk::High,
        Disposition::Deny,
        Reason::DestructiveWithoutGrant,
    );
    assert_eq!(decision.classification().reversible, Some(false));
    assert_eq!(decision.classification().effect, Effect::Destructive);
    let decision = expect(
        ProtectedWithApprovalClaim,
        Risk::Critical,
        Disposition::Deny,
        Reason::ProtectedTarget,
    );
    assert_eq!(decision.classification().reversible, Some(false));
    let mut protected = fixtures::load(ProtectedWithApprovalClaim).proposal;
    protected.requestor = None;
    protected.native = Native::Denied;
    assert_eq!(
        classify_proposal(&protected).reason,
        Reason::ProtectedTarget
    );
}

#[test]
fn at03_pipeline_and_unknown_tool_stay_opaque() {
    for fixture in [Pipeline, UnknownTool] {
        let decision = expect(
            fixture,
            Risk::Unknown,
            Disposition::Review,
            Reason::OpaqueEffects,
        );
        assert_eq!(decision.classification().effect, Effect::Unknown);
        assert_eq!(decision.classification().gap, Some(EvidenceGap::Effects));
    }
}

#[test]
fn at04_powershell_dynamic_input_is_never_a_candidate() {
    let decision = expect(
        DynamicPowerShell,
        Risk::Unknown,
        Disposition::Review,
        Reason::OpaqueEffects,
    );
    assert!(
        !format!("{decision:?}", decision = decision.classification()).contains("EncodedCommand")
    );
    let mut disguised = fixtures::load(ScopedRead).proposal;
    disguised.command = fixtures::load(DynamicPowerShell).proposal.command;
    assert_eq!(classify_proposal(&disguised).effect, Effect::Unknown);
    assert_eq!(
        classify_proposal(&disguised).disposition,
        Disposition::Review
    );
}

#[test]
fn at05_structured_nested_context_denies_known_protected_target() {
    let proposal = fixtures::load(CmdPowerShellProtected).proposal;
    assert_eq!(
        proposal.arguments,
        ["cmd /c", "powershell -Command", "$env:FIXTURE_TARGET"]
    );
    expect(
        CmdPowerShellProtected,
        Risk::Critical,
        Disposition::Deny,
        Reason::ProtectedTarget,
    );
    let decision = expect(
        CmdPowerShellUnknown,
        Risk::Unknown,
        Disposition::Review,
        Reason::OpaqueEffects,
    );
    assert_eq!(decision.classification().target, None);
}

#[test]
fn at06_python_identity_does_not_hide_subprocess_or_script_change() {
    let before = expect(
        PythonSubprocess,
        Risk::Unknown,
        Disposition::Review,
        Reason::OpaqueEffects,
    );
    let after = expect(
        ChangedPythonScript,
        Risk::Unknown,
        Disposition::Review,
        Reason::OpaqueEffects,
    );
    assert_eq!(before.classification(), after.classification());
    assert!(!before.matches_proposal(&after.proposal));
}

#[test]
fn at07_npm_lifecycle_is_high_risk_and_read_only_control_stays_low() {
    let decision = expect(
        NpmLifecycle,
        Risk::High,
        Disposition::Review,
        Reason::LifecycleMayRun,
    );
    assert_eq!(decision.classification().effect, Effect::LifecycleMayRun);
    assert_eq!(decision.classification().gap, Some(EvidenceGap::Effects));
    expect(
        ScopedRead,
        Risk::Low,
        Disposition::SyntheticCandidate,
        Reason::ScopedRead,
    );
}

#[test]
fn at19_changes_to_each_input_break_the_old_binding() {
    let decision = classify(ScopedRead);
    let base = fixtures::load(ScopedRead).proposal;
    assert!(decision.matches_proposal(&base));
    let changes = [
        (
            "recipe",
            Proposal {
                action: Action::BatchDelete,
                ..base
            },
        ),
        (
            "scope",
            Proposal {
                scope: Scope::OutsideFixtureRoot,
                ..base
            },
        ),
        (
            "target",
            Proposal {
                target: Some("fixture://workspace/Source.rs"),
                ..base
            },
        ),
        (
            "object",
            Proposal {
                object: Some("changed-object"),
                ..base
            },
        ),
        (
            "native",
            Proposal {
                native: Native::Denied,
                ..base
            },
        ),
        (
            "command",
            Proposal {
                command: Some("changed-command"),
                ..base
            },
        ),
        (
            "arguments",
            Proposal {
                arguments: &["other.rs"],
                ..base
            },
        ),
        (
            "cwd",
            Proposal {
                cwd: "fixture://other",
                ..base
            },
        ),
        (
            "requestor",
            Proposal {
                requestor: Some("changed-requestor"),
                ..base
            },
        ),
        (
            "executor",
            Proposal {
                executor: Some("changed-executor"),
                ..base
            },
        ),
        (
            "runtime",
            Proposal {
                runtime: "changed-runtime",
                ..base
            },
        ),
        (
            "provider",
            Proposal {
                provider: "changed-provider",
                ..base
            },
        ),
        (
            "config",
            Proposal {
                config: "changed-config",
                ..base
            },
        ),
        (
            "session",
            Proposal {
                session: "changed-session",
                ..base
            },
        ),
        (
            "invocation",
            Proposal {
                invocation: "changed-invocation",
                ..base
            },
        ),
        (
            "executable",
            Proposal {
                executable: "changed-executable",
                ..base
            },
        ),
        (
            "script",
            Proposal {
                script: Some("changed-script"),
                ..base
            },
        ),
        (
            "environment",
            Proposal {
                environment: "changed-environment",
                ..base
            },
        ),
        (
            "stdin",
            Proposal {
                stdin: Some("changed-stdin"),
                ..base
            },
        ),
        (
            "network",
            Proposal {
                network: Some("changed-network"),
                ..base
            },
        ),
        (
            "policy_epoch",
            Proposal {
                policy_epoch: 2,
                ..base
            },
        ),
        (
            "grant_epoch",
            Proposal {
                grant_epoch: 2,
                ..base
            },
        ),
        (
            "core_epoch",
            Proposal {
                core_epoch: "changed-core-epoch",
                ..base
            },
        ),
    ];
    for (field, proposal) in changes {
        assert!(
            !decision.matches_proposal(&proposal),
            "changed {field} accepted"
        );
    }
}

#[test]
fn at26_model_and_mcp_text_have_no_authority_or_echo() {
    let hard = expect(
        ProtectedWithModelInstruction,
        Risk::Critical,
        Disposition::Deny,
        Reason::ProtectedTarget,
    );
    let deny = expect(
        NativeDenyWithModelInstruction,
        Risk::Unknown,
        Disposition::Deny,
        Reason::NativeDenied,
    );
    let unknown = expect(
        UnknownNativeWithModelInstruction,
        Risk::Unknown,
        Disposition::Review,
        Reason::NativePermissionUnknown,
    );
    for decision in [hard, deny, unknown] {
        assert!(!format!("{:?}", decision.classification()).contains("fixture-secret-value"));
        assert_ne!(
            decision.classification().disposition,
            Disposition::SyntheticCandidate
        );
    }
}

#[test]
fn fixture_results_are_deterministic_and_only_one_is_a_candidate() {
    let cases = [
        ScopedRead,
        MissingRequestor,
        MissingObject,
        OutsideRead,
        BatchDelete,
        ProtectedWithApprovalClaim,
        ProtectedWithModelInstruction,
        Pipeline,
        DynamicPowerShell,
        CmdPowerShellProtected,
        CmdPowerShellUnknown,
        PythonSubprocess,
        ChangedPythonScript,
        NpmLifecycle,
        NativeDenyWithModelInstruction,
        UnknownNativeWithModelInstruction,
        UnknownTool,
    ];
    let first = cases.map(classify);
    for (case, expected) in cases.into_iter().zip(first).rev() {
        let result = classify(case);
        assert!(result == expected, "classification changed for {case:?}");
        assert_eq!(
            result.classification().disposition == Disposition::SyntheticCandidate,
            case == ScopedRead
        );
    }
}
