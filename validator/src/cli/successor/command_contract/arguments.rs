use std::path::{Path, PathBuf};

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
    PackageRoot,
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
            Self::PackageRoot => "--package-root",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Flag,
    Identifier,
    RelativePath,
    HostPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptionSpec {
    pub name: OptionName,
    pub kind: ValueKind,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelativePath(pub(crate) String);

impl RelativePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPath(pub(crate) PathBuf);

impl HostPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedValue {
    Flag,
    Identifier(String),
    RelativePath(RelativePath),
    HostPath(HostPath),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionArgument {
    pub name: OptionName,
    pub value: ParsedValue,
}
