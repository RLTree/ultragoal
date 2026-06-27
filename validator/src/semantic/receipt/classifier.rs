use crate::audit::contract::{ProofGate, SemanticClass};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn classify(claim: &Value, text: &str) -> Vec<SemanticClass> {
    let tokens = crate::claim::text::tokens(text);
    let mut classes = Vec::new();
    if has_any(&tokens, &["dashboard", "dashboards"]) {
        classes.push(SemanticClass::Dashboard);
    }
    if text.contains("run console") || text.contains("run-console") {
        classes.push(SemanticClass::RunConsole);
    }
    if text.contains("workflow launcher") || text.contains("workflow-launcher") {
        classes.push(SemanticClass::WorkflowLauncher);
    }
    if has_any(&tokens, &["gui", "browser", "desktop", "native"])
        || app_surface_text(&tokens)
        || text.contains("desktop client")
        || text.contains("native client")
    {
        classes.push(SemanticClass::LocalAppDesktopBrowser);
    }
    if text.contains("control surface")
        || text.contains("settings surface")
        || text.contains("control panel")
        || text.contains("visual workbench")
        || text.contains("visual workspace")
        || text.contains("visual client")
        || text.contains("visual console")
        || text.contains("graphical workbench")
        || text.contains("graphical shell")
        || has_any(&tokens, &["controls", "settings", "panel"])
    {
        classes.push(SemanticClass::UiControlSurface);
    }
    if crate::claim::language::install_visibility_claim(text, &tokens) {
        classes.push(SemanticClass::InstallVisibleSelectableActiveInCodex);
    }
    if crate::claim::language::publication_claim(text, &tokens) {
        classes.push(SemanticClass::PublicationMarketplaceCatalogWorkspaceRegistry);
    }
    if product_applicable(claim, text, &tokens) {
        classes.push(SemanticClass::ProductUserFacingSurface);
    }
    if classes.is_empty() {
        classes.push(SemanticClass::RuntimeCliBackendOnlyEngineOnly);
    }
    classes.sort();
    classes.dedup();
    classes
}

pub fn proof_gates(classes: &[SemanticClass]) -> Vec<ProofGate> {
    let mut gates = vec![ProofGate::RuntimeExecution, ProofGate::ReadyForMergeReceipt];
    if classes.iter().any(product_class) {
        gates.extend([
            ProofGate::ProductCohesionReceipt,
            ProofGate::UiJourneyEvidence,
            ProofGate::AccessibilityEvidence,
        ]);
    }
    if classes.contains(&SemanticClass::AmbiguousNeedsReviewerClassification) {
        gates.push(ProofGate::ReviewerClassification);
    }
    if classes.contains(&SemanticClass::InstallVisibleSelectableActiveInCodex) {
        gates.push(ProofGate::InstallVisibilityReceipt);
    }
    if classes.contains(&SemanticClass::PublicationMarketplaceCatalogWorkspaceRegistry) {
        gates.push(ProofGate::PublicationExternalAttestation);
    }
    gates.sort();
    gates.dedup();
    gates
}

fn product_applicable(claim: &Value, text: &str, tokens: &BTreeSet<String>) -> bool {
    let app = &claim["product_applicability"];
    ["user_facing", "product_surface", "ui_or_control_surface"]
        .iter()
        .any(|key| app.get(*key).and_then(Value::as_bool) == Some(true))
        || crate::claim::language::product_surface_claim(text, tokens)
        || has_any(
            tokens,
            &[
                "user",
                "customer",
                "operator",
                "consumer",
                "frontend",
                "front-end",
            ],
        )
}

fn product_class(class: &SemanticClass) -> bool {
    matches!(
        class,
        SemanticClass::ProductUserFacingSurface
            | SemanticClass::UiControlSurface
            | SemanticClass::LocalAppDesktopBrowser
            | SemanticClass::Dashboard
            | SemanticClass::RunConsole
            | SemanticClass::WorkflowLauncher
    )
}

fn has_any(tokens: &BTreeSet<String>, values: &[&str]) -> bool {
    values.iter().any(|value| tokens.contains(*value))
}

fn app_surface_text(tokens: &BTreeSet<String>) -> bool {
    has_any(tokens, &["app", "application"])
        && has_any(
            tokens,
            &[
                "available",
                "complete",
                "installed",
                "launched",
                "ready",
                "usable",
                "visible",
                "works",
            ],
        )
        && !has_any(tokens, &["backend", "cli", "engine", "headless", "server"])
}
