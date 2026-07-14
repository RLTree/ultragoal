use super::ParseOutcome;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRoot {
    path: PathBuf,
}

impl WorkspaceRoot {
    pub(crate) fn workspace_default() -> Self {
        Self {
            path: PathBuf::from("."),
        }
    }

    pub(crate) fn from_option_value(value: &str) -> Option<Self> {
        (!value.trim().is_empty()).then(|| Self {
            path: PathBuf::from(value),
        })
    }

    pub(crate) fn into_path_buf(self) -> PathBuf {
        self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedCommandLine {
    root: WorkspaceRoot,
    outcome: ParseOutcome,
}

impl ParsedCommandLine {
    pub(crate) const fn new(root: WorkspaceRoot, outcome: ParseOutcome) -> Self {
        Self { root, outcome }
    }

    pub fn into_parts(self) -> (WorkspaceRoot, ParseOutcome) {
        (self.root, self.outcome)
    }
}
