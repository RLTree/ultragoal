use super::super::error::ParseErrorId;
use std::path::{Path, PathBuf};

const MAX_HOST_PATH_BYTES: usize = 4096;

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
    RetainIsolatedRoot,
    RoutineConfig,
    LocalState,
    InterruptAfter,
    Continuation,
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
            Self::RetainIsolatedRoot => "--retain-isolated-root",
            Self::RoutineConfig => "--routine-config",
            Self::LocalState => "--local-state",
            Self::InterruptAfter => "--interrupt-after",
            Self::Continuation => "--continuation",
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
    pub(crate) fn parse(value: &str) -> Result<Self, ParseErrorId> {
        let path = Self(value.into());
        if path.is_valid() {
            Ok(path)
        } else {
            Err(ParseErrorId::InvalidPath)
        }
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn is_valid(&self) -> bool {
        Self::is_valid_path(&self.0)
    }

    pub(crate) fn is_valid_path(path: &Path) -> bool {
        path.to_str().is_some_and(|value| {
            !value.is_empty()
                && value.len() <= MAX_HOST_PATH_BYTES
                && !value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
                && Path::new(value).is_absolute()
        })
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
