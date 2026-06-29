use std::collections::BTreeSet;

pub(crate) fn install_visibility_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    contains_any(
        text,
        &[
            "agent picker",
            "custom agent appears",
            "from the chooser",
            "install visibility",
            "listed in the chooser",
            "plugin chooser",
            "tools menu",
            "visible after install",
        ],
    ) || subject_action_surface(
        tokens,
        &["addon", "add-on", "agent", "extension", "plugin", "tool"],
        &[
            "active",
            "appears",
            "available",
            "chosen",
            "discoverable",
            "listed",
            "present",
            "registered",
            "selectable",
            "visible",
        ],
        &["chooser", "codex", "menu", "picker", "store"],
    ) || (has_any(tokens, &["app", "application"])
        && has_any(tokens, &["available", "installed", "visible"])
        && !has_any(tokens, &["backend", "cli", "engine", "headless", "server"]))
        || (tokens.contains("install") && has_any(tokens, &["visibility", "visible"]))
        || (has_any(tokens, &["install", "installation"])
            && has_any(tokens, &["button", "succeeded", "success", "works"]))
}

pub(crate) fn publication_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    let split_marketplace = tokens.contains("market") && tokens.contains("place");
    has_any(
        tokens,
        &["marketplace", "catalog", "directory", "gallery", "registry"],
    ) || contains_any(
        text,
        &[
            "codex store",
            "community index",
            "market place",
            "public catalog",
            "public directory",
            "shared workspace index",
            "team library",
            "team shelf",
            "workspace registry",
        ],
    ) || split_marketplace
        || subject_action_surface(
            tokens,
            &["addon", "add-on", "extension", "module", "plugin", "tool"],
            &[
                "browse",
                "browsable",
                "discoverable",
                "listed",
                "published",
                "released",
            ],
            &[
                "catalog",
                "directory",
                "gallery",
                "index",
                "library",
                "market",
                "marketplace",
                "place",
                "registry",
                "shelf",
            ],
        )
}

pub(crate) fn dogfood_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    contains_any(text, &["dog food", "dogfood", "multi lane", "multi-lane"])
        && has_any(tokens, &["dogfood", "dog", "food", "lane"])
}

pub(crate) fn external_product_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    contains_any(
        text,
        &["external product", "downbeat ux", "codex workflow rs ux"],
    ) || (tokens.contains("external") && has_any(tokens, &["product", "ux"]))
}

pub(crate) fn live_runtime_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    contains_any(text, &["live runtime"])
        || (tokens.contains("live") && has_any(tokens, &["runtime", "e2e"]))
}

pub(crate) fn product_surface_claim(text: &str, tokens: &BTreeSet<String>) -> bool {
    crate::claim::language::product::product_surface_claim(text, tokens)
}

pub(crate) fn app_surface_completion(tokens: &BTreeSet<String>) -> bool {
    crate::claim::language::product::app_surface_completion(tokens)
}

fn subject_action_surface(
    tokens: &BTreeSet<String>,
    subjects: &[&str],
    actions: &[&str],
    surfaces: &[&str],
) -> bool {
    has_any(tokens, subjects) && has_any(tokens, actions) && has_any(tokens, surfaces)
}

fn contains_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn has_any(tokens: &BTreeSet<String>, values: &[&str]) -> bool {
    values.iter().any(|value| tokens.contains(*value))
}

#[cfg(test)]
mod tests {
    use super::{
        external_product_claim, install_visibility_claim, live_runtime_claim,
        product_surface_claim, publication_claim,
    };

    fn text_and_tokens(value: &str) -> (String, std::collections::BTreeSet<String>) {
        let text = crate::claim::text::normalized_text(&[value]);
        let tokens = crate::claim::text::tokens(&text);
        (text, tokens)
    }

    #[test]
    fn semantic_claim_language_routes_product_live_install_and_publication_claims() {
        for value in [
            "external product proof exists",
            "Downbeat UX claim",
            "external UX launch",
        ] {
            let (text, tokens) = text_and_tokens(value);
            assert!(external_product_claim(&text, &tokens), "{value}");
        }
        for value in ["live runtime proof exists", "live e2e is proven"] {
            let (text, tokens) = text_and_tokens(value);
            assert!(live_runtime_claim(&text, &tokens), "{value}");
        }
        let (text, tokens) = text_and_tokens("plugin listed in the chooser");
        assert!(install_visibility_claim(&text, &tokens));
        let (text, tokens) = text_and_tokens("plugin published in the market place");
        assert!(publication_claim(&text, &tokens));
        let (text, tokens) = text_and_tokens("operator can launch the control surface");
        assert!(product_surface_claim(&text, &tokens));
    }
}
pub(crate) mod product;
