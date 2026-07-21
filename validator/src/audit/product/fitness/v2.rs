use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = string(receipt, "/target_revision/value");
    operator_and_class(receipt, &mut out);
    surface_identities(receipt, &candidate, &mut out);
    public_entry(receipt, &candidate, &mut out);
    real_work(receipt, &candidate, &mut out);
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

fn surface_identities(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
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
        let evidence = item.pointer("/evidence").unwrap_or(&Value::Null);
        let path = string(evidence, "/path");
        if identity.is_empty() || path.is_empty() {
            out.push(format!(
                "product_fitness_surface_observation_missing:{name}"
            ));
        }
        bound_evidence(
            evidence,
            candidate,
            &format!("surface_identities/{name}"),
            out,
        );
        if !identities.insert(identity) || !paths.insert(path) {
            out.push(format!("product_fitness_surface_wrong_identity:{name}"));
        }
    }
}

fn public_entry(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
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
    bound_evidence(
        receipt
            .pointer("/public_entry_observation/evidence")
            .unwrap_or(&Value::Null),
        candidate,
        "public_entry_observation",
        out,
    );
}

fn real_work(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let repository = string(receipt, "/real_work_observation/repository_identity");
    let valid_digest = repository
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    if !valid_digest {
        out.push("product_fitness_real_repository_missing".to_string());
    }
    let repository_evidence = receipt
        .pointer("/real_work_observation/repository_evidence")
        .unwrap_or(&Value::Null);
    if string(repository_evidence, "/digest") != repository {
        out.push("product_fitness_real_repository_unbound".to_string());
    }
    bound_evidence(
        repository_evidence,
        candidate,
        "real_work_observation/repository",
        out,
    );
    bound_evidence(
        receipt
            .pointer("/real_work_observation/evidence")
            .unwrap_or(&Value::Null),
        candidate,
        "real_work_observation",
        out,
    );
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
        "time_to_verified_value_ms",
        "human_interventions",
        "review_rounds",
        "failure",
        "diagnosis",
        "recovery_outcome",
        "repeat_use_outcome",
        "retained_artifact_bytes",
        "retained_cache_bytes",
        "false_passes",
        "false_rejections",
    ] {
        let value = receipt.pointer(&format!("/manual_journey_row/{field}"));
        if value.is_none_or(|value| match value {
            Value::String(value) => value.trim().is_empty(),
            Value::Number(value) => value.as_u64().is_none(),
            _ => true,
        }) {
            out.push(format!("product_fitness_manual_journey_missing:{field}"));
        }
    }
}

fn ceiling(receipt: &Value, out: &mut Vec<String>) {
    if string(receipt, "/claim_ceiling") != "live_same_surface_proven" {
        return;
    }
    let class = string(receipt, "/evidence_class");
    if class != "repeated_human_use"
        || string(receipt, "/surface_identities/journey/status") != "observed"
    {
        out.push("product_fitness_unsupported_claim_ceiling".to_string());
    }
    for layer in [
        "source",
        "package",
        "marketplace",
        "install",
        "cache",
        "app_registry",
        "discovery",
        "runtime",
        "journey",
    ] {
        if string(receipt, &format!("/surface_identities/{layer}/status")) == "observed"
            && !supports_live_layer(layer, &class)
        {
            out.push("product_fitness_unsupported_claim_ceiling".to_string());
        }
    }
}

fn bound_evidence(evidence: &Value, candidate: &str, name: &str, out: &mut Vec<String>) {
    if string(evidence, "/path").is_empty()
        || string(evidence, "/digest").is_empty()
        || string(evidence, "/candidate_id") != candidate
        || evidence.get("same_surface").and_then(Value::as_bool) != Some(true)
        || evidence.get("current_session").and_then(Value::as_bool) != Some(true)
    {
        out.push(format!("product_fitness_evidence_unbound:{name}"));
    }
}

fn supports_live_layer(layer: &str, class: &str) -> bool {
    match class {
        "source" => layer == "source",
        "package" => matches!(layer, "source" | "package"),
        "installed" => matches!(layer, "source" | "package" | "marketplace" | "install"),
        "runtime" | "agent_use" => matches!(
            layer,
            "source"
                | "package"
                | "marketplace"
                | "install"
                | "cache"
                | "app_registry"
                | "discovery"
                | "runtime"
        ),
        "human_use" | "repeated_human_use" => matches!(
            layer,
            "source"
                | "package"
                | "marketplace"
                | "install"
                | "cache"
                | "app_registry"
                | "discovery"
                | "runtime"
                | "journey"
        ),
        _ => false,
    }
}

fn string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
