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
    Orchestration,
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
