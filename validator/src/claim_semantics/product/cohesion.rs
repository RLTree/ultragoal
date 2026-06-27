use crate::audit::contract::Failure;
use crate::json_boundary;
use serde_json::Value;
use std::collections::BTreeSet;

const UI_SURFACES: &[&str] = &["ui_browser", "ui_computer"];
const UI_EVIDENCE: &[&str] = &["ui_interaction", "live_beneficial_e2e"];
const NON_PRODUCT_CLASSIFICATIONS: &[&str] = &[
    "engine_runtime_only",
    "internal_non_product",
    "proof_surface_negative_probe",
];
const PRODUCT_CLASSIFICATIONS: &[&str] = &[
    "consumer_product",
    "dashboard",
    "local_app",
    "workflow_launcher",
    "run_console",
    "control_surface",
];
pub fn product_cohesion_failures(claim: &Value) -> Vec<Failure> {
    let mut out = Vec::new();
    let context = product_context(claim);
    included_applicability_failures(claim, &context, &mut out);
    feature_product_applicability_check(claim, &mut out);
    explicit_product_evidence_checks(claim, &context, &mut out);
    out
}

fn included_applicability_failures(
    claim: &Value,
    context: &ProductContext,
    out: &mut Vec<Failure>,
) {
    if str_field(claim, "claim_ceiling_effect") != "included"
        || !context.product_applicable
        || context.explicit_product
    {
        return;
    }
    let error = applicability_error(claim, context);
    out.push(fail(error, str_field(claim, "id")));
}

fn applicability_error(claim: &Value, context: &ProductContext) -> &'static str {
    if context.text_product
        && claim.get("product_applicability").is_some()
        && context.product_flags.is_empty()
    {
        "product_applicability_contradicts_claim_text"
    } else if !context.product_flags.is_empty() {
        "product_applicability_requires_cohesion"
    } else if context.text_product {
        "product_claim_text_requires_cohesion"
    } else if claim.get("product_cohesion_waiver").is_some() && context.ui_applicable {
        "product_cohesion_waiver_on_consumer_ui_claim"
    } else {
        "product_cohesion_applicability_missing"
    }
}

fn explicit_product_evidence_checks(
    claim: &Value,
    context: &ProductContext,
    out: &mut Vec<Failure>,
) {
    if !context.explicit_product || str_field(claim, "claim_ceiling_effect") != "included" {
        return;
    }
    let evidence = claim
        .get("evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let product = evidence
        .iter()
        .filter(|ev| {
            str_field(ev, "kind") == "product_cohesion_receipt"
                && str_field(ev, "surface") == "product::cohesion"
        })
        .collect::<Vec<_>>();
    let ui = evidence
        .iter()
        .filter(|ev| {
            UI_SURFACES.contains(&str_field(ev, "surface").as_str())
                && UI_EVIDENCE.contains(&str_field(ev, "kind").as_str())
        })
        .collect::<Vec<_>>();
    if product.is_empty() {
        out.push(fail(
            "product_cohesion_receipt_missing",
            str_field(claim, "id"),
        ));
    }
    if ui.is_empty() {
        out.push(fail(
            "product_ui_journey_evidence_missing",
            str_field(claim, "id"),
        ));
    }
    let evidence_paths = evidence
        .iter()
        .filter_map(|ev| ev.get("path").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let ui_paths = ui
        .iter()
        .filter_map(|ev| ev.get("path").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for ev in product {
        crate::claim_semantics::product::receipt::policy::product_receipt_evidence_checks(
            ev,
            &evidence_paths,
            &ui_paths,
            out,
        );
    }
}

fn product_context(claim: &Value) -> ProductContext {
    let product_applicability = claim.get("product_applicability");
    let product_flags = product_applicability_flags(product_applicability);
    let text_product = claim_text_product_applicable(claim);
    let explicit_product = bool_field(claim, "requires_product_cohesion")
        || str_field(claim, "claim_kind") == "product::cohesion"
        || str_field(claim, "claim_surface") == "product::cohesion";
    let ui_applicable = str_field(claim, "claim_kind") == "ui_surface"
        || UI_SURFACES.contains(&str_field(claim, "claim_surface").as_str())
        || array_strings(claim, "allowed_evidence_surfaces")
            .iter()
            .any(|s| UI_SURFACES.contains(&s.as_str()));
    ProductContext {
        product_applicable: ui_applicable
            || explicit_product
            || !product_flags.is_empty()
            || text_product,
        product_flags,
        text_product,
        explicit_product,
        ui_applicable,
    }
}

struct ProductContext {
    product_flags: BTreeSet<String>,
    text_product: bool,
    explicit_product: bool,
    ui_applicable: bool,
    product_applicable: bool,
}

pub fn claim_text_product_applicable(claim: &Value) -> bool {
    crate::claim_semantics::product::text::policy::claim_text_product_applicable(claim)
}

fn feature_product_applicability_check(claim: &Value, out: &mut Vec<Failure>) {
    if str_field(claim, "claim_kind") != "feature_completion"
        || str_field(claim, "claim_ceiling_effect") != "included"
    {
        return;
    }
    let Some(applicability) = claim.get("product_applicability") else {
        out.push(fail(
            "product_applicability_missing",
            str_field(claim, "id"),
        ));
        return;
    };
    let flags = product_applicability_flags(Some(applicability));
    let classification = str_field(applicability, "surface_classification");
    if flags.is_empty() && !NON_PRODUCT_CLASSIFICATIONS.contains(&classification.as_str()) {
        out.push(fail(
            "product_non_product_classification_missing",
            str_field(claim, "id"),
        ));
    }
    if !flags.is_empty() && NON_PRODUCT_CLASSIFICATIONS.contains(&classification.as_str()) {
        out.push(fail(
            "product_applicability_conflicting_classification",
            str_field(claim, "id"),
        ));
    }
}

fn product_applicability_flags(value: Option<&Value>) -> BTreeSet<String> {
    let mut flags = BTreeSet::new();
    let Some(value) = value else {
        return flags;
    };
    for key in ["user_facing", "product_surface", "ui_or_control_surface"] {
        if bool_field(value, key) {
            flags.insert(key.to_string());
        }
    }
    if PRODUCT_CLASSIFICATIONS.contains(&str_field(value, "surface_classification").as_str()) {
        flags.insert("surface_classification".to_string());
    }
    flags
}

pub(super) fn fail(error: &str, detail: impl Into<String>) -> Failure {
    Failure::new("product-cohesion-proof", error, detail)
}

fn str_field(value: &Value, key: &str) -> String {
    json_boundary::string(value, key).unwrap_or_default()
}

fn bool_field(value: &Value, key: &str) -> bool {
    json_boundary::bool_value(value, key).unwrap_or(false)
}

fn array_strings(value: &Value, key: &str) -> Vec<String> {
    json_boundary::string_array(value, key)
}
