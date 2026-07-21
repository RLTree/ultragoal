use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    operator_and_class(receipt, &mut out);
    surface_identities(receipt, &mut out);
    public_entry(receipt, &mut out);
    real_work(receipt, &mut out);
    manual_journey(receipt, &mut out);
    ceiling(receipt, &mut out);
    out
}

fn operator_and_class(receipt: &Value, out: &mut Vec<String>) {
    let operator = string(receipt, "/operator_kind");
    let class = string(receipt, "/evidence_class");
    if operator.is_empty() || class.is_empty() {
        out.push("product_fitness_v2_observation_missing:operator_or_class".to_string());
        return;
    }
    let supervised = operator == "agent" || operator == "agent_with_human_supervision";
    if supervised && matches!(class.as_str(), "human_use" | "repeated_human_use") {
        out.push("product_fitness_agent_use_relabeled_as_human_use".to_string());
    }
    if operator == "human" && class == "agent_use" {
        out.push("product_fitness_agent_use_relabeled_as_human_use".to_string());
    }
    let continuance = receipt
        .pointer("/continuance_signal/required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let repeated = receipt
        .pointer("/claim/repeated_use_claimed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if class == "agent_use" && (continuance || repeated) {
        out.push("product_fitness_agent_use_relabeled_as_continuance".to_string());
    }
    if repeated && class != "repeated_human_use" {
        out.push("product_fitness_continuance_missing".to_string());
    }
}

fn surface_identities(receipt: &Value, out: &mut Vec<String>) {
    let Some(surfaces) = receipt.pointer("/surface_identities") else {
        out.push("product_fitness_v2_observation_missing:surface_identities".to_string());
        return;
    };
    let names = [
        "source",
        "package",
        "marketplace",
        "install",
        "cache",
        "app_registry",
        "discovery",
        "runtime",
        "journey",
    ];
    let mut identities = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for name in names {
        let item = surfaces.get(name).unwrap_or(&Value::Null);
        let status = string(item, "/status");
        if status == "withheld" {
            if item.get("identity").is_some() || item.get("evidence").is_some() {
                out.push(format!(
                    "product_fitness_surface_observation_invalid:{name}"
                ));
            }
            continue;
        }
        if status != "observed" {
            out.push(format!(
                "product_fitness_surface_observation_missing:{name}"
            ));
            continue;
        }
        let identity = string(item, "/identity");
        let path = string(item, "/evidence/path");
        if identity.is_empty() || path.is_empty() {
            out.push(format!(
                "product_fitness_surface_observation_missing:{name}"
            ));
        }
        if !identities.insert(identity) || !paths.insert(path) {
            out.push(format!("product_fitness_surface_wrong_identity:{name}"));
        }
    }
}

fn public_entry(receipt: &Value, out: &mut Vec<String>) {
    if string(receipt, "/public_entry_observation/surface_id") != "PS-ENTRY"
        || string(receipt, "/public_entry_observation/route") != "harness-ultragoal"
    {
        out.push("product_fitness_public_entry_wrong_surface".to_string());
    }
    let attempted = receipt
        .pointer("/public_entry_observation/bypass_attempted")
        .and_then(Value::as_bool);
    let rejected = receipt
        .pointer("/public_entry_observation/bypass_rejected")
        .and_then(Value::as_bool);
    match (attempted, rejected) {
        (Some(true), Some(true) | Some(false)) if rejected == Some(false) => {
            out.push("product_fitness_bypass_not_rejected".to_string())
        }
        (Some(_), Some(_)) => {}
        _ => out.push("product_fitness_public_entry_observation_missing".to_string()),
    }
}

fn real_work(receipt: &Value, out: &mut Vec<String>) {
    let repository = string(receipt, "/real_work_observation/repository_identity");
    let valid_digest = repository
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    if !valid_digest {
        out.push("product_fitness_real_repository_missing".to_string());
    }
    for field in ["task_id", "task", "useful_outcome"] {
        if string(receipt, &format!("/real_work_observation/{field}"))
            .trim()
            .is_empty()
        {
            out.push(format!("product_fitness_real_work_missing:{field}"));
        }
    }
}

fn manual_journey(receipt: &Value, out: &mut Vec<String>) {
    for field in [
        "failure",
        "diagnosis",
        "recovery_outcome",
        "repeat_use_outcome",
    ] {
        if string(receipt, &format!("/manual_journey_row/{field}"))
            .trim()
            .is_empty()
        {
            out.push(format!("product_fitness_manual_journey_missing:{field}"));
        }
    }
}

fn ceiling(receipt: &Value, out: &mut Vec<String>) {
    if string(receipt, "/claim_ceiling") != "live_same_surface_proven" {
        return;
    }
    if string(receipt, "/surface_identities/journey/status") != "observed" {
        out.push("product_fitness_unsupported_claim_ceiling".to_string());
    }
    if string(receipt, "/evidence_class") == "agent_use" {
        out.push("product_fitness_unsupported_claim_ceiling".to_string());
    }
}

fn string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
