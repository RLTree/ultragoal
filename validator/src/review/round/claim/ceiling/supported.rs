const SUPPORTED: &[&str] = &[
    "package_static_fixture_proof",
    "detached_review_target_archive_identity",
];
const UNSUPPORTED: &[&str] = &[
    "plugins_ui_visibility",
    "install_button_success",
    "workspace_public_marketplace_publication",
    "real_multilane_dogfood",
    "production_readiness",
    "external_product_ux_improvement",
    "reviewer_runtime_configuration",
    "custom_agent_runtime_discovery",
];

pub(crate) fn row_authority_errors(
    root: &Path,
    receipt: &Value,
    anchors: &AnchorValues,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    counterexample_errors(root, row, persona, out);
    proof_anchor_errors(root, anchors, row, persona, out);
    next_repair_errors(root, row, persona, out);
    claim_ceiling_errors(receipt, row, persona, out);
}

fn counterexample_errors(root: &Path, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let probes = row
        .get("counterexamples_attempted")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if probes.len() < 3 {
        out.push(failure("review_round_counterexamples_missing", persona));
    }
    let mut keys = BTreeSet::new();
    for probe in probes {
        let key = format!(
            "{}|{}",
            probe.get("probe_id").and_then(Value::as_str).unwrap_or(""),
            probe
                .get("counterexample")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        if !keys.insert(key) {
            out.push(failure("review_round_counterexamples_missing", persona));
        }
        crate::review::round::artifacts::artifact_ref_error(
            root,
            probe.get("evidence"),
            persona,
            "counterexamples_attempted",
            out,
        );
    }
}

fn proof_anchor_errors(
    root: &Path,
    anchors: &AnchorValues,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let rows = row
        .get("proof_anchors_checked")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let checked_paths = rows
        .iter()
        .filter_map(|row| row.get("path").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let Some(spec) = crate::review::round::config::review_role_spec(persona) else {
        return;
    };
    let agent_role = crate::review::round::config::agent_role(spec);
    let prompt_packet = row
        .pointer("/prompt_packet/path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let external_anchors = [
        (&anchors.validator_path, &anchors.validator_digest),
        (&anchors.review_target_path, &anchors.review_target_digest),
        (&anchors.archive_path, &anchors.archive_digest),
    ];
    for artifact in &rows {
        if !external_anchor_matches(artifact, &external_anchors) {
            crate::review::round::artifacts::artifact_ref_error(
                root,
                Some(artifact),
                persona,
                "proof_anchors_checked",
                out,
            );
        }
    }
    let required_anchors = [
        anchors.validator_path.as_str(),
        anchors.review_target_path.as_str(),
        anchors.archive_path.as_str(),
        agent_role.manifest_path,
        prompt_packet,
        spec.focus_path,
    ];
    for required in required_anchors {
        if !checked_paths.contains(required) {
            out.push(failure("review_round_report_too_shallow", persona));
            return;
        }
    }
}

fn external_anchor_matches(artifact: &Value, anchors: &[(&String, &String); 3]) -> bool {
    let Some(path) = artifact.get("path").and_then(Value::as_str) else {
        return false;
    };
    let Some(digest) = artifact.get("digest").and_then(Value::as_str) else {
        return false;
    };
    anchors.iter().any(|(want_path, want_digest)| {
        path == want_path.as_str() && digest == want_digest.as_str()
    })
}

fn next_repair_errors(root: &Path, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let rows = row
        .get("required_next_repairs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        out.push(failure("review_round_report_too_shallow", persona));
    }
    for repair in rows {
        crate::review::round::artifacts::artifact_ref_error(
            root,
            repair.get("evidence"),
            persona,
            "required_next_repairs",
            out,
        );
    }
}
