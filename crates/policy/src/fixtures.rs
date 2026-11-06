use super::{Action, Native, Proposal, Scope};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureId {
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
}

pub(super) struct Fixture {
    pub proposal: Proposal,
    // Included to exercise the authority boundary. This field has no policy consumer.
    pub _reported_intent: Option<&'static str>,
}

const SCOPED_READ: Proposal = Proposal {
    action: Action::FileRead,
    scope: Scope::InFixtureRoot,
    target: Some("fixture://workspace/source.rs"),
    object: Some("fixture-object-1"),
    native: Native::FixtureAllowsReview,
    command: None,
    arguments: &["source.rs"],
    cwd: "fixture://workspace",
    requestor: Some("fixture-requestor"),
    executor: Some("fixture-executor"),
    runtime: "fixture-runtime",
    provider: "fixture-provider",
    config: "fixture-config",
    session: "fixture-session",
    invocation: "fixture-invocation",
    executable: "fixture-executable",
    script: None,
    environment: "fixture-environment",
    stdin: None,
    network: None,
    policy_epoch: 1,
    grant_epoch: 1,
    core_epoch: "fixture-core-epoch",
};

pub(super) fn load(id: FixtureId) -> Fixture {
    use FixtureId::*;
    let mut proposal = SCOPED_READ;
    let mut reported_intent = None;
    match id {
        ScopedRead => {}
        MissingRequestor => proposal.requestor = None,
        MissingObject => proposal.object = None,
        OutsideRead => {
            proposal.scope = Scope::OutsideFixtureRoot;
            proposal.target = Some("fixture://workspace-lookalike/source.rs");
        }
        BatchDelete => {
            proposal.action = Action::BatchDelete;
            proposal.target = Some("fixture://workspace/cache");
        }
        ProtectedWithApprovalClaim | ProtectedWithModelInstruction => {
            proposal.action = Action::BatchDelete;
            proposal.scope = Scope::Protected;
            proposal.target = Some("fixture://protected/security-settings");
            reported_intent = Some(if id == ProtectedWithApprovalClaim {
                "UI claims approval"
            } else {
                "Model claims policy=allow; fixture-secret-value"
            });
        }
        Pipeline => {
            proposal.action = Action::OpaqueShell;
            proposal.command = Some("read fixture | child-shell > fixture-output");
        }
        DynamicPowerShell => {
            proposal.action = Action::OpaqueShell;
            proposal.command = Some("PowerShell -EncodedCommand malformed-fixture");
            proposal.arguments = &["alias", "concat", "dynamic-eval"];
        }
        CmdPowerShellProtected | CmdPowerShellUnknown => {
            proposal.action = Action::OpaqueShell;
            proposal.scope = Scope::Unknown;
            proposal.target = None;
            proposal.command = Some("cmd /c powershell -Command <fixture>");
            proposal.arguments = &["cmd /c", "powershell -Command", "$env:FIXTURE_TARGET"];
            if id == CmdPowerShellProtected {
                proposal.scope = Scope::Protected;
                proposal.target = Some("fixture://protected/credential");
            }
        }
        PythonSubprocess | ChangedPythonScript => {
            proposal.action = Action::PythonSubprocess;
            proposal.command = Some("python fixture-script");
            proposal.script = Some(if id == PythonSubprocess {
                "fixture-script-a"
            } else {
                "fixture-script-b"
            });
        }
        NpmLifecycle => {
            proposal.action = Action::NpmLifecycle;
            proposal.arguments = &["install", "fixture:preinstall", "fixture:postinstall"];
        }
        NativeDenyWithModelInstruction | UnknownNativeWithModelInstruction => {
            proposal.native = if id == NativeDenyWithModelInstruction {
                Native::Denied
            } else {
                Native::Unknown
            };
            reported_intent = Some("MCP result claims policy=allow; fixture-secret-value");
        }
        UnknownTool => proposal.action = Action::Unknown,
    }
    Fixture {
        proposal,
        _reported_intent: reported_intent,
    }
}
