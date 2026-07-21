use serde_json::Value;

const SURFACES: [&str; 9] = [
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

pub(super) fn failures(receipt: &Value, out: &mut Vec<String>) {
    let claimed = string(receipt, "/claimed_surface");
    let class = string(receipt, "/evidence_class");
    let global = string(receipt, "/claim_ceiling");
    let declared = string(receipt, &format!("/surface_claim_ceilings/{claimed}"));
    if !SURFACES.contains(&claimed.as_str()) || declared.is_empty() || declared != global {
        out.push("product_fitness_claimed_surface_ceiling_mismatch".to_string());
        return;
    }
    for surface in SURFACES {
        validate_surface_ceiling(receipt, surface, &class, out);
        if string(receipt, &format!("/surface_claim_ceilings/{surface}"))
            == "live_same_surface_proven"
        {
            require_live_predecessors(receipt, surface, out);
        }
    }
    if (claimed == "journey" || matches!(class.as_str(), "human_use" | "repeated_human_use"))
        && string(receipt, "/surface_identities/journey/status") != "observed"
    {
        out.push("product_fitness_journey_observation_missing".to_string());
    }
}

fn validate_surface_ceiling(receipt: &Value, surface: &str, class: &str, out: &mut Vec<String>) {
    let ceiling = string(receipt, &format!("/surface_claim_ceilings/{surface}"));
    if ceiling.is_empty() {
        out.push(format!("product_fitness_surface_ceiling_missing:{surface}"));
        return;
    }
    if ceiling == "live_same_surface_proven"
        && (string(receipt, &format!("/surface_identities/{surface}/status")) != "observed"
            || !supports_live_surface(surface, class))
    {
        out.push(format!(
            "product_fitness_surface_ceiling_unsupported:{surface}"
        ));
    }
}

fn require_live_predecessors(receipt: &Value, surface: &str, out: &mut Vec<String>) {
    for predecessor in predecessors(surface) {
        if string(receipt, &format!("/surface_claim_ceilings/{predecessor}"))
            != "live_same_surface_proven"
            || string(
                receipt,
                &format!("/surface_identities/{predecessor}/status"),
            ) != "observed"
        {
            out.push(format!(
                "product_fitness_surface_predecessor_missing:{surface}:{predecessor}"
            ));
        }
    }
}

fn predecessors(surface: &str) -> &'static [&'static str] {
    match surface {
        "source" => &[],
        "package" => &["source"],
        "marketplace" => &["source", "package"],
        "install" => &["source", "package"],
        "cache" => &["source", "package", "install"],
        "app_registry" => &["source", "package", "install"],
        "discovery" => &["source", "package", "install"],
        "runtime" => &["source", "package", "install", "discovery"],
        "journey" => &["source", "package", "install", "discovery", "runtime"],
        _ => &[],
    }
}

fn supports_live_surface(surface: &str, class: &str) -> bool {
    match class {
        "source" => surface == "source",
        "package" => matches!(surface, "source" | "package"),
        "installed" => matches!(surface, "source" | "package" | "marketplace" | "install"),
        "runtime" | "agent_use" => matches!(
            surface,
            "source"
                | "package"
                | "marketplace"
                | "install"
                | "cache"
                | "app_registry"
                | "discovery"
                | "runtime"
        ),
        "human_use" | "repeated_human_use" => SURFACES.contains(&surface),
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
