fn full_errors(value: &Value, out: &mut Vec<String>) {
    let reviewers = strings(value, "reviewers_required");
    if ROLES.iter().any(|spec| !reviewers.contains(spec.role_name))
        || reviewers.len() != ROLES.len()
    {
        out.push("materiality_full_scope_reviewer_set_missing".into());
    }
    no_signoff(value, out, "materiality_reviewer_cannot_authorize_signoff");
}

fn delta_errors(value: &Value, out: &mut Vec<String>) {
    no_signoff(value, out, "materiality_delta_used_for_signoff");
    let count = strings(value, "reviewers_required").len();
    if count > 1 {
        out.push("materiality_delta_reviewer_scope_too_broad".into());
    } else if count == 0 {
        out.push("materiality_delta_reviewer_scope_invalid".into());
    }
}

fn advisory_errors(value: &Value, out: &mut Vec<String>) {
    no_signoff(value, out, "materiality_advisory_used_for_signoff");
    if !strings(value, "reviewers_required").is_empty()
        || value
            .pointer("/claim_ceiling/supported")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
            != 0
    {
        out.push("materiality_advisory_overclaims_support".into());
    }
}

fn blocked_errors(value: &Value, out: &mut Vec<String>) {
    if !strings(value, "reviewers_required").is_empty() {
        out.push("materiality_blocked_launches_reviewers".into());
    }
    if strings(value, "validator_repairs_recommended").is_empty() {
        out.push("materiality_blocked_without_repair".into());
    }
}

fn no_signoff(value: &Value, out: &mut Vec<String>, error: &str) {
    if value["output_may_be_used_for_material_signoff"].as_bool() != Some(false) {
        out.push(error.into());
    }
}

fn strings(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn text_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn failure(error: &str) -> ReviewFailure {
    ReviewFailure::new("material-review-scope-gate", error, "materiality_gate")
}
