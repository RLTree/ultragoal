use super::super::{ITEM_LIMIT, PluginInterface, bounded_text, https, unique_list};
use super::SemanticIssue;
use std::collections::BTreeSet;

pub(super) fn validate(interface: &PluginInterface, issues: &mut BTreeSet<SemanticIssue>) {
    let texts = [
        interface.display_name.as_deref(),
        interface.short_description.as_deref(),
        interface.long_description.as_deref(),
        interface.developer_name.as_deref(),
        interface.category.as_deref(),
    ];
    let assets = [
        interface.composer_icon.as_deref(),
        interface.logo.as_deref(),
        interface.logo_dark.as_deref(),
    ];
    if texts
        .into_iter()
        .flatten()
        .any(|value| !bounded_text(value))
        || assets
            .into_iter()
            .flatten()
            .any(|value| !bounded_text(value))
        || !unique_list(&interface.capabilities, ITEM_LIMIT)
        || !unique_list(&interface.screenshots, ITEM_LIMIT)
    {
        issues.insert(SemanticIssue::InterfaceMetadata);
    }
    let urls = [
        interface.website_url.as_deref(),
        interface.privacy_policy_url.as_deref(),
        interface.terms_url.as_deref(),
    ];
    if urls.into_iter().flatten().any(|value| !https(value)) {
        issues.insert(SemanticIssue::Url);
    }
    if interface
        .default_prompt
        .as_ref()
        .is_some_and(|values| values.is_empty() || !unique_list(values, ITEM_LIMIT))
    {
        issues.insert(SemanticIssue::DefaultPrompt);
    } else if interface.default_prompt.as_ref().is_some_and(|values| {
        values.len() > 3 || values.iter().any(|value| value.chars().count() > 128)
    }) {
        issues.insert(SemanticIssue::UnsupportedDefaultPrompt);
    }
    if interface.brand_color.as_deref().is_some_and(|value| {
        value.len() != 7
            || !value.starts_with('#')
            || !value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        issues.insert(SemanticIssue::BrandColor);
    }
}
