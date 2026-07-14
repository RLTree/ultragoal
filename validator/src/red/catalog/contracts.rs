use serde::Serialize;
use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct RedCatalogProjectionRequest<'a> {
    pub(crate) root: &'a Path,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExpectedFailure {
    pub(crate) check_id: String,
    pub(crate) error: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RedCatalogRow {
    pub(crate) expected_failure: ExpectedFailure,
    pub(crate) id: String,
    pub(crate) packet_digest: String,
    pub(crate) packet_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RedCatalogProjection {
    pub(crate) bytes: Vec<u8>,
    pub(crate) rows: Vec<RedCatalogRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RedCatalogError {
    code: &'static str,
    path: Option<String>,
}

impl RedCatalogError {
    pub(super) fn new(code: &'static str, path: Option<String>) -> Self {
        Self { code, path }
    }

    pub(crate) fn stable_text(&self) -> String {
        self.path.as_ref().map_or_else(
            || self.code.to_string(),
            |path| format!("{}:{path}", self.code),
        )
    }
}
