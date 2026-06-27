use std::path::Path;

pub(super) fn anti_theater_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    law: &str,
) -> Vec<String> {
    if !is_anti_theater_law(law) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for failure in crate::audit::final_packet::package_failures(root, store) {
        out.push(format!(
            "mandatory_law_anti_theater_dependency:{law}:{failure}"
        ));
    }
    for failure in crate::audit::plugin::registry::failures(root, store) {
        out.push(format!(
            "mandatory_law_anti_theater_dependency:{law}:{failure}"
        ));
    }
    for failure in cli_control_receipt_failures(root) {
        out.push(format!(
            "mandatory_law_anti_theater_dependency:{law}:{failure}"
        ));
    }
    out
}

fn cli_control_receipt_failures(root: &Path) -> Vec<String> {
    [
        (
            "validation_artifacts/cli/update-goal-eligibility.json",
            "update_goal_eligibility",
        ),
        (
            "validation_artifacts/cli/self-law-receipt.json",
            "self_update_goal_eligibility",
        ),
    ]
    .into_iter()
    .flat_map(|(path, operation)| receipt_file_failures(root, path, operation))
    .collect()
}

fn receipt_file_failures(root: &Path, path: &str, operation: &str) -> Vec<String> {
    let value = match crate::json_boundary::read_json(&root.join(path)) {
        Ok(value) => value,
        Err(_) => return vec![format!("cli_control_plane_receipt_missing:{path}")],
    };
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    crate::cli::control::plane::receipt::same_candidate_pass_failures(&value, &expected, operation)
}

fn is_anti_theater_law(law: &str) -> bool {
    matches!(
        law,
        "generated-proof-artifact-provenance-anti-fabrication"
            | "adversarial-packet-tampering-forged-proof-rejection"
    )
}
