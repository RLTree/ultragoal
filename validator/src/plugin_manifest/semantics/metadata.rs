use super::super::{PluginManifest, bounded_text, email, https, kebab, semver, unique_list};
use super::SemanticIssue;
use std::collections::BTreeSet;

pub(super) fn validate(manifest: &PluginManifest, issues: &mut BTreeSet<SemanticIssue>) {
    let author = manifest.author.as_ref();
    let texts = [
        Some(manifest.description.as_str()),
        author.map(|value| value.name.as_str()),
        manifest.license.as_deref(),
    ];
    if !kebab(&manifest.name)
        || !semver(&manifest.version)
        || texts
            .into_iter()
            .flatten()
            .any(|value| !bounded_text(value))
        || !unique_list(&manifest.keywords, super::super::ITEM_LIMIT)
    {
        issues.insert(SemanticIssue::Metadata);
    }
    if author
        .and_then(|value| value.email.as_deref())
        .is_some_and(|value| !email(value))
    {
        issues.insert(SemanticIssue::AuthorEmail);
    }
    let urls = [
        manifest.homepage.as_deref(),
        manifest.repository.as_deref(),
        author.and_then(|value| value.url.as_deref()),
    ];
    if urls.into_iter().flatten().any(|value| !https(value)) {
        issues.insert(SemanticIssue::Url);
    }
}
