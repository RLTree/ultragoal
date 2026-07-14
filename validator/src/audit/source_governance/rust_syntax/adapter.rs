use super::model::{AuthorityUse, FunctionShape, IdentifierDeclaration};
use super::{use_aliases, visitor};
use std::collections::BTreeSet;

pub(crate) struct RustSyntaxRequest<'a> {
    pub(crate) source_path: &'a str,
    pub(crate) source_bytes: &'a [u8],
    pub(crate) include_test_items: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustSyntaxReport {
    pub(crate) authorities: BTreeSet<AuthorityUse>,
    pub(crate) closed_records: BTreeSet<String>,
    pub(crate) disallowed_lint_allowances: BTreeSet<String>,
    pub(crate) external_roots: BTreeSet<String>,
    pub(crate) functions: Vec<FunctionShape>,
    pub(crate) identifiers: BTreeSet<IdentifierDeclaration>,
    pub(crate) lint_metadata_failures: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustSyntaxError {
    pub(crate) source_path: String,
    pub(crate) detail: String,
}

impl RustSyntaxError {
    pub(crate) fn stable_text(&self) -> String {
        format!(
            "rust_syntax_parse_failed:{}:{}",
            self.source_path, self.detail
        )
    }
}

pub(crate) fn analyze(request: RustSyntaxRequest<'_>) -> Result<RustSyntaxReport, RustSyntaxError> {
    let text = std::str::from_utf8(request.source_bytes).map_err(|error| RustSyntaxError {
        source_path: request.source_path.to_string(),
        detail: format!("non_utf8:{error}"),
    })?;
    let file = syn::parse_file(text).map_err(|error| RustSyntaxError {
        source_path: request.source_path.to_string(),
        detail: error.to_string(),
    })?;
    let aliases = use_aliases::collect(&file);
    Ok(visitor::inspect(&file, aliases, request.include_test_items))
}
