use serde::Serialize;
use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct CliSelfLawCheckRequest<'a> {
    pub(crate) root: &'a Path,
    pub(crate) jobs: usize,
}

impl<'a> CliSelfLawCheckRequest<'a> {
    pub(crate) const fn new(root: &'a Path, jobs: usize) -> Self {
        Self { root, jobs }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct CliSelfLawFinding {
    pub(crate) check_id: String,
    pub(crate) detail: String,
}

#[derive(Debug)]
pub(crate) struct CliSelfLawCheckResponse {
    findings: Vec<CliSelfLawFinding>,
}

impl CliSelfLawCheckResponse {
    pub(super) fn new(findings: Vec<CliSelfLawFinding>) -> Self {
        Self { findings }
    }

    pub(crate) fn into_findings(self) -> Vec<CliSelfLawFinding> {
        self.findings
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CliSelfLawCheckError {
    InvalidParallelism,
}

impl CliSelfLawCheckError {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::InvalidParallelism => "cli_self_law_parallelism_invalid",
        }
    }
}
