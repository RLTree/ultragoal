use crate::audit::contract::{ClassifierKind, SemanticClassificationReceipt};
use crate::digest;
use crate::json_boundary;
use serde_json::Value;
use std::path::PathBuf;

pub struct GenerateOptions {
    pub root: PathBuf,
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub implementation_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub contract_id: String,
    pub contract_version: String,
    pub prompt_contract_digest: Option<String>,
    pub producer_actor_id: String,
    pub classifier_actor_id: String,
}

pub fn generate(options: GenerateOptions) -> Result<(), String> {
    let input_path = if options.input.is_absolute() {
        options.input.clone()
    } else {
        options.root.join(&options.input)
    };
    let manifest = json_boundary::read_json(&input_path)?;
    std::fs::create_dir_all(&options.out_dir).map_err(|err| {
        format!(
            "{}: create receipt dir failed: {err}",
            options.out_dir.display()
        )
    })?;
    let claims = manifest
        .get("claims")
        .or_else(|| manifest.pointer("/completion_manifest/claims"))
        .and_then(Value::as_array)
        .ok_or_else(|| "completion manifest claims must be a list".to_string())?;
    for claim in claims {
        let receipt = build_receipt(claim, &options)?;
        let claim_id = claim.get("id").and_then(Value::as_str).unwrap_or("claim");
        let path = options.out_dir.join(format!(
            "{}.semantic-classification-receipt.json",
            sanitize(claim_id)
        ));
        json_boundary::write_json(
            &path,
            &serde_json::to_value(receipt).expect("semantic receipt serialization is derived"),
        )?;
    }
    Ok(())
}

pub fn build_receipt(
    claim: &Value,
    options: &GenerateOptions,
) -> Result<SemanticClassificationReceipt, String> {
    let text = claim_text_for_claim(claim);
    let classes = crate::semantic::receipt::classifier::classify(claim, &text);
    let gates = crate::semantic::receipt::classifier::proof_gates(&classes);
    let kind = classifier_kind(&options.implementation_kind)?;
    if kind != ClassifierKind::DeterministicBackstop {
        let provider = options.provider.as_deref().unwrap_or("<none>");
        let model = options.model.as_deref().unwrap_or("<none>");
        return Err(format!(
            "{kind:?} receipts require external model or reviewer proof; this generator only mints deterministic_backstop receipts; supplied provider={provider} model={model}"
        ));
    }
    let mut receipt = SemanticClassificationReceipt {
        schema: "harness-ultragoal.semantic-classification-receipt.v1".to_string(),
        claim_id: claim
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        canonical_text_digest: digest::bytes(text.as_bytes()),
        classifier_contract_id: options.contract_id.clone(),
        classifier_contract_version: options.contract_version.clone(),
        classifier_implementation_kind: kind,
        provider_model: None,
        prompt_contract_digest: options.prompt_contract_digest.clone(),
        classifier_evidence: None,
        generated_at: crate::audit::clock::now_iso(),
        producer_actor_id: options.producer_actor_id.clone(),
        classifier_actor_id: options.classifier_actor_id.clone(),
        actor_disjoint: options.producer_actor_id != options.classifier_actor_id,
        detected_semantic_classes: classes,
        rationale: "Deterministic lexical backstop over enumerated semantic families.".to_string(),
        confidence: 0.82,
        ambiguity: false,
        required_proof_gates: gates,
        claim_ceiling_recommendation: "validate_required_gates_before_inclusion".to_string(),
        receipt_digest: digest::ZERO.to_string(),
    };
    receipt.receipt_digest = receipt_digest(&receipt);
    Ok(receipt)
}

pub fn claim_text_for_claim(claim: &Value) -> String {
    crate::claim::text::normalized_text(&[
        claim.get("title").and_then(Value::as_str).unwrap_or(""),
        claim
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
    ])
}

fn classifier_kind(value: &str) -> Result<ClassifierKind, String> {
    match value {
        "model" => Ok(ClassifierKind::Model),
        "human_reviewer" => Ok(ClassifierKind::HumanReviewer),
        "deterministic_backstop" => Ok(ClassifierKind::DeterministicBackstop),
        _ => Err(format!("unknown classifier implementation kind: {value}")),
    }
}

fn receipt_digest(receipt: &SemanticClassificationReceipt) -> String {
    let mut value =
        serde_json::to_value(receipt).expect("semantic receipt serialization is derived");
    value["receipt_digest"] = Value::String(digest::ZERO.to_string());
    digest::canonical_json(&value)
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}
pub(crate) mod classifier;
#[cfg(test)]
mod tests;
