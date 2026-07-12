mod interface;
mod mcp;
mod metadata;

use super::PluginManifest;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SemanticIssue {
    Metadata,
    AuthorEmail,
    Url,
    InterfaceMetadata,
    DefaultPrompt,
    UnsupportedDefaultPrompt,
    BrandColor,
    McpServer,
}

pub(crate) fn semantic_issues(manifest: &PluginManifest) -> BTreeSet<SemanticIssue> {
    let mut issues = BTreeSet::new();
    metadata::validate(manifest, &mut issues);
    if let Some(value) = &manifest.interface {
        interface::validate(value, &mut issues);
    }
    if let Some(value) = &manifest.mcp_servers {
        mcp::validate(value, &mut issues);
    }
    issues
}
