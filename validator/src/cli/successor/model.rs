use crate::context::EffectClass;

pub const fn effect_name(effect: EffectClass) -> &'static str {
    match effect {
        EffectClass::Read => "read",
        EffectClass::PlannedWrite => "planned-write",
        EffectClass::WorkspaceWrite => "workspace-write",
        EffectClass::ExternalWrite => "external-write",
        EffectClass::Destructive => "destructive",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Group {
    Inspect,
    Next,
    Fit,
    Check,
    Diagnose,
    Prove,
    Observe,
    Package,
    Eval,
    Migrate,
}

impl Group {
    pub const ALL: [Self; 10] = [
        Self::Inspect,
        Self::Next,
        Self::Fit,
        Self::Check,
        Self::Diagnose,
        Self::Prove,
        Self::Observe,
        Self::Package,
        Self::Eval,
        Self::Migrate,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Next => "next",
            Self::Fit => "fit",
            Self::Check => "check",
            Self::Diagnose => "diagnose",
            Self::Prove => "prove",
            Self::Observe => "observe",
            Self::Package => "package",
            Self::Eval => "eval",
            Self::Migrate => "migrate",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|group| group.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectTarget {
    Summary,
    Context,
    Inventory,
    Capabilities,
    Findings,
    Claims,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FitAction {
    Inspect,
    Plan,
    Apply,
    Verify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckProfile {
    Routine,
    Strict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserveAction {
    Query,
    Export,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageAction {
    Inventory,
    Build,
    Verify,
    InstallTest,
    Publish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvalAction {
    Audit,
    Run,
    Harvest,
    Promote,
    Adapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrateAction {
    Plan,
    Apply,
    Verify,
    Retire,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuccessorCommand {
    Inspect(InspectTarget),
    Next,
    Fit(FitAction),
    Check(CheckProfile),
    Diagnose,
    Prove,
    Observe(ObserveAction),
    Package(PackageAction),
    Eval(EvalAction),
    Migrate(MigrateAction),
}

impl SuccessorCommand {
    pub const fn group(self) -> Group {
        match self {
            Self::Inspect(_) => Group::Inspect,
            Self::Next => Group::Next,
            Self::Fit(_) => Group::Fit,
            Self::Check(_) => Group::Check,
            Self::Diagnose => Group::Diagnose,
            Self::Prove => Group::Prove,
            Self::Observe(_) => Group::Observe,
            Self::Package(_) => Group::Package,
            Self::Eval(_) => Group::Eval,
            Self::Migrate(_) => Group::Migrate,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OptionName {
    Target,
    Plan,
    AcceptPlan,
    Claim,
    Finding,
    Filter,
    Output,
    ApproveExport,
    Provider,
    ApprovePublish,
    Spec,
    Input,
    Candidate,
    Registry,
    ApproveRetirement,
}

impl OptionName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Target => "--target",
            Self::Plan => "--plan",
            Self::AcceptPlan => "--accept-plan",
            Self::Claim => "--claim",
            Self::Finding => "--finding",
            Self::Filter => "--filter",
            Self::Output => "--output",
            Self::ApproveExport => "--approve-export",
            Self::Provider => "--provider",
            Self::ApprovePublish => "--approve-publish",
            Self::Spec => "--spec",
            Self::Input => "--input",
            Self::Candidate => "--candidate",
            Self::Registry => "--registry",
            Self::ApproveRetirement => "--approve-retirement",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Flag,
    Identifier,
    RelativePath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptionSpec {
    pub name: OptionName,
    pub kind: ValueKind,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandDescriptor {
    pub command: SuccessorCommand,
    pub subcommand: Option<&'static str>,
    pub effect: EffectClass,
    pub purpose: &'static str,
    pub options: &'static [OptionSpec],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelativePath(pub(crate) String);

impl RelativePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedValue {
    Flag,
    Identifier(String),
    RelativePath(RelativePath),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionArgument {
    pub name: OptionName,
    pub value: ParsedValue,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputMode {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedInvocation {
    pub command: SuccessorCommand,
    pub effect: EffectClass,
    pub output_mode: OutputMode,
    pub arguments: Vec<OptionArgument>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpTarget {
    Root,
    Group(Group),
    Command(SuccessorCommand),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    Invocation(ParsedInvocation),
    Help {
        target: HelpTarget,
        output_mode: OutputMode,
    },
    Version(OutputMode),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitClass {
    Success,
    ActionableFinding,
    InvalidInvocation,
    BlockedAuthority,
    UnsupportedCapability,
    InternalFailure,
}

impl ExitClass {
    pub const fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::ActionableFinding => 1,
            Self::InvalidInvocation => 2,
            Self::BlockedAuthority => 3,
            Self::UnsupportedCapability => 4,
            Self::InternalFailure => 70,
        }
    }
}
