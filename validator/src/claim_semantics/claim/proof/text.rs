use crate::audit::contract::Failure;
use crate::claim_semantics::{evidence, good_status, str_field};
use serde_json::Value;
use std::collections::BTreeSet;

type TextProofRequirement = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static str],
);

const TEXT_PROOF_REQUIREMENTS: &[TextProofRequirement] = &[
    (
        "install_visibility",
        &[
            "codex app install visibility",
            "custom agent appears in agent picker",
            "extension is present in codex",
            "extension registered in codex",
            "add-on is selectable",
            "addon is selectable",
            "install visibility",
            "installed app visible",
            "harness ultragoal appears in codex",
            "plugin appears in plugin list",
            "plugin available in codex store",
            "plugin installed from codex store",
            "plugin installed and visible",
            "plugin picked from codex store",
            "tools menu after reload",
            "shows up in agent picker",
            "appears in codex",
            "shows up inside codex",
            "chosen inside codex",
            "from the chooser in codex",
            "install button works",
            "install button succeeded",
            "install succeeded",
            "installation succeeded",
            "app is available",
            "app is installed",
            "app is visible",
            "application is available",
            "application is installed",
            "application is visible",
        ],
        &["external"],
        &["promotion_receipt"],
    ),
    (
        "publication",
        &[
            "marketplace publication",
            "plugin appears in marketplace search",
            "plugin is listed in marketplace",
            "plugin market place release",
            "plugin published in workspace registry",
            "market place release",
            "public catalog",
            "public directory",
            "workspace gallery",
            "workspace registry",
            "community index",
            "team library",
            "shared workspace index",
            "team's add-on index",
        ],
        &["external"],
        &["promotion_receipt"],
    ),
    (
        "dogfood",
        &["multi lane dogfood", "multi-lane dogfood", "real dogfood"],
        &["root_integration"],
        &["dogfood_receipt"],
    ),
    (
        "external_product_ux",
        &[
            "codex-workflow-rs ux",
            "downbeat ux",
            "external product cohesion",
        ],
        &["product::cohesion", "ui_browser", "ui_computer", "external"],
        &[
            "product_cohesion_receipt",
            "ui_interaction",
            "external_attestation",
        ],
    ),
];

pub fn text_surface_checks(claim: &Value, out: &mut Vec<Failure>) {
    if str_field(claim, "claim_ceiling_effect") != "included"
        && !good_status(&str_field(claim, "status"))
    {
        return;
    }
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    let tokens = crate::claim::text::tokens(&text);
    let evs = evidence(claim);
    for (label, terms, required_surfaces, required_evidence) in TEXT_PROOF_REQUIREMENTS {
        if text_requires_proof(&text, &tokens, label, terms)
            && !proof_row_exists(&evs, required_surfaces, required_evidence)
        {
            out.push(Failure::new(
                "claim-status-ceiling",
                "claim_text_requires_unproven_surface",
                format!("{}:{label}", str_field(claim, "id")),
            ));
            return;
        }
    }
}

pub(crate) fn proof_row_exists(
    evs: &[&Value],
    required_surfaces: &[&str],
    required_evidence: &[&str],
) -> bool {
    evs.iter().any(|ev| {
        required_surfaces.contains(&str_field(ev, "surface").as_str())
            && required_evidence.contains(&str_field(ev, "kind").as_str())
    })
}

fn text_requires_proof(text: &str, tokens: &BTreeSet<String>, label: &str, terms: &[&str]) -> bool {
    crate::claim::text::contains_any(text, terms) || tokens_match_requirement(label, tokens)
}

fn tokens_match_requirement(label: &str, tokens: &BTreeSet<String>) -> bool {
    if label == "install_visibility" {
        return install_tokens_match(tokens);
    }
    if label == "publication" {
        return publication_tokens_match(tokens);
    }
    false
}

fn install_tokens_match(tokens: &BTreeSet<String>) -> bool {
    let text = format!(" {} ", tokens.iter().cloned().collect::<Vec<_>>().join(" "));
    if crate::claim::language::install_visibility_claim(&text, tokens) {
        return true;
    }
    visible_plugin(tokens)
        || codex_plugin_selection(tokens)
        || visible_app(tokens)
        || install_success(tokens)
}

fn visible_plugin(tokens: &BTreeSet<String>) -> bool {
    crate::claim_semantics::intersects(
        tokens,
        &[
            "active",
            "activated",
            "appears",
            "enabled",
            "present",
            "registered",
            "selectable",
            "visible",
            "chosen",
        ],
    ) && crate::claim_semantics::intersects(tokens, &["addon", "extension", "plugin", "ultragoal"])
}

fn codex_plugin_selection(tokens: &BTreeSet<String>) -> bool {
    tokens.contains("codex")
        && crate::claim_semantics::intersects(tokens, &["loaded", "picked", "store", "chosen"])
        && crate::claim_semantics::intersects(tokens, &["addon", "extension", "plugin"])
}

fn visible_app(tokens: &BTreeSet<String>) -> bool {
    crate::claim_semantics::intersects(tokens, &["app", "application"])
        && crate::claim_semantics::intersects(tokens, &["available", "installed", "visible"])
        && !crate::claim_semantics::intersects(
            tokens,
            &["backend", "cli", "engine", "headless", "server"],
        )
}

fn install_success(tokens: &BTreeSet<String>) -> bool {
    let install_button = tokens.contains("install")
        && tokens.contains("button")
        && crate::claim_semantics::intersects(tokens, &["work", "works", "succeeded", "success"]);
    let install_success = crate::claim_semantics::intersects(tokens, &["install", "installation"])
        && crate::claim_semantics::intersects(tokens, &["succeeded", "success", "works"]);
    install_button || install_success
}

fn publication_tokens_match(tokens: &BTreeSet<String>) -> bool {
    let text = format!(" {} ", tokens.iter().cloned().collect::<Vec<_>>().join(" "));
    if crate::claim::language::publication_claim(&text, tokens) {
        return true;
    }
    let surface = ["marketplace", "catalog", "directory", "gallery", "registry"];
    let split_marketplace = tokens.contains("market") && tokens.contains("place");
    (split_marketplace
        || crate::claim_semantics::intersects(tokens, &surface)
        || contextual_publication_surface(tokens)
        || bare_publication(tokens))
        && crate::claim_semantics::intersects(
            tokens,
            &["listed", "published", "search", "discoverable", "released"],
        )
}

fn bare_publication(tokens: &BTreeSet<String>) -> bool {
    crate::claim_semantics::intersects(
        tokens,
        &["addon", "add-on", "extension", "module", "plugin", "tool"],
    ) && crate::claim_semantics::intersects(tokens, &["published", "released"])
}

fn contextual_publication_surface(tokens: &BTreeSet<String>) -> bool {
    crate::claim_semantics::intersects(tokens, &["index", "library"])
        && crate::claim_semantics::intersects(
            tokens,
            &["addon", "add-on", "extension", "module", "plugin"],
        )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    fn tokens(text: &str) -> BTreeSet<String> {
        crate::claim::text::tokens(text)
    }

    #[test]
    fn fallback_token_predicates_remain_explicit() {
        assert!(!super::visible_app(&tokens("backend app visible")));
        assert!(super::install_success(&tokens("install button works")));
        assert!(super::install_success(&tokens("installation success")));
        assert!(super::contextual_publication_surface(&tokens(
            "library module"
        )));
        assert!(super::publication_tokens_match(&tokens("plugin published")));
        assert!(super::publication_tokens_match(&tokens(
            "marketplace released"
        )));
    }
}
