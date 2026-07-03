pub(crate) fn failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    if rel.starts_with("validator/src/audit/namespace/")
        || rel.starts_with("validator/src/self_tests/namespace/")
        || !line.contains('"')
    {
        return None;
    }
    let label = string_literals(line)
        .iter()
        .find_map(|literal| super::path_labels::product_opaque_goal_work_string_label(literal))?;
    if owned_template_project_state_rejection_catalog(rel, line, &label) {
        return None;
    }
    Some(format!(
        "namespace_validator_source_product_opaque_goal_work_string:path={rel};line={line_number};label={label};why=string_ids_or_diagnostics_name_goal_work_or_evidence_posture_instead_of_product_behavior;repair=rename_string_id_or_diagnostic_by_cli_product_behavior_or_authority_object;claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
    ))
}

fn owned_template_project_state_rejection_catalog(rel: &str, line: &str, label: &str) -> bool {
    rel == "validator/src/audit/template_integrity.rs"
        && label.starts_with("session_history_")
        && line.trim_start().starts_with('"')
}

fn string_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if !in_string {
            if ch == '"' {
                in_string = true;
                current.clear();
            }
            continue;
        }
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            out.push(std::mem::take(&mut current));
            in_string = false;
            continue;
        }
        current.push(ch);
    }
    out
}
