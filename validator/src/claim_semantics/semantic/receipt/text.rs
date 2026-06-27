use crate::audit::contract::{Failure, SemanticClass, SemanticClassificationReceipt};
use crate::claim_semantics::claim::proof;
use crate::claim_semantics::str_field;
use serde_json::Value;

pub(super) fn text_receipt_backstop(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    let text = proof::claim_text(claim);
    let tokens = crate::claim::text::tokens(&text);
    if crate::claim::language::install_visibility_claim(&text, &tokens)
        && !receipt
            .detected_semantic_classes
            .contains(&SemanticClass::InstallVisibleSelectableActiveInCodex)
    {
        push(
            out,
            "semantic_classification_contradicts_claim_text",
            claim,
            "install_visibility",
        );
    }
    if crate::claim::language::publication_claim(&text, &tokens)
        && !receipt
            .detected_semantic_classes
            .contains(&SemanticClass::PublicationMarketplaceCatalogWorkspaceRegistry)
    {
        push(
            out,
            "semantic_classification_contradicts_claim_text",
            claim,
            "publication",
        );
    }
}

pub(super) fn is_product_class(class: &SemanticClass) -> bool {
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

pub(super) fn deterministic_backstop_lower_bound(
    claim: &Value,
    receipt: &SemanticClassificationReceipt,
    out: &mut Vec<Failure>,
) {
    let text = proof::claim_text(claim);
    for class in crate::semantic::receipt::classifier::classify(claim, &text) {
        if !receipt.detected_semantic_classes.contains(&class) {
            push(
                out,
                "semantic_classification_contradicts_claim_text",
                claim,
                "deterministic_backstop",
            );
            return;
        }
    }
}

fn push(out: &mut Vec<Failure>, error: &str, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        error,
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
