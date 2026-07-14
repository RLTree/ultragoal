fn claim_ceiling_errors(receipt: &Value, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let Some(assessment) = row.get("claim_ceiling_assessment") else {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    };
    if assessment.get("authority").and_then(Value::as_str)
        != Some("falsification_only_cannot_raise")
    {
        out.push(failure(
            "review_round_claim_ceiling_authority_invalid",
            persona,
        ));
        return;
    }
    let top_supported = claim_ids(&receipt["claim_ceiling"], "supported");
    let top_unsupported = claim_ids(&receipt["claim_ceiling"], "unsupported");
    let row_not_disproven = claim_ids(assessment, "not_disproven");
    let row_challenged = claim_ids(assessment, "challenged");
    if row_challenged.is_empty() {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    }
    if has_duplicates(assessment, "not_disproven") || has_duplicates(assessment, "challenged") {
        out.push(failure("review_round_claim_ceiling_duplicate", persona));
        return;
    }
    if unsupported_claim_in_supported(&top_supported)
        || unsupported_claim_in_supported(&row_not_disproven)
        || !same_set(&top_supported, SUPPORTED)
        || !row_not_disproven.is_subset(&top_supported)
    {
        out.push(failure("review_round_claim_ceiling_overclaim", persona));
        return;
    }
    if !same_set(&top_unsupported, UNSUPPORTED) {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    }
    let all_top = top_supported
        .union(&top_unsupported)
        .copied()
        .collect::<BTreeSet<_>>();
    let all_row = row_not_disproven
        .union(&row_challenged)
        .copied()
        .collect::<BTreeSet<_>>();
    if !top_unsupported.is_subset(&row_challenged)
        || !all_top.is_subset(&all_row)
        || !row_not_disproven.is_disjoint(&row_challenged)
    {
        out.push(failure("review_round_claim_ceiling_mismatch", persona));
    }
}

fn claim_ids<'a>(value: &'a Value, key: &str) -> BTreeSet<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .collect()
}

fn has_duplicates(value: &Value, key: &str) -> bool {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .map(normalize)
        .collect::<Vec<_>>();
    values.len() != values.iter().collect::<BTreeSet<_>>().len()
}

fn same_set(values: &BTreeSet<&str>, expected: &[&str]) -> bool {
    values == &expected.iter().copied().collect::<BTreeSet<_>>()
}

fn unsupported_claim_in_supported(values: &BTreeSet<&str>) -> bool {
    values
        .iter()
        .filter(|value| !SUPPORTED.contains(value))
        .any(|value| {
            let normalized = crate::claim::text::normalized_text(&[value]);
            let tokens = crate::claim::text::tokens(&normalized);
            crate::claim::language::install_visibility_claim(&normalized, &tokens)
                || crate::claim::language::publication_claim(&normalized, &tokens)
                || crate::claim::language::dogfood_claim(&normalized, &tokens)
                || crate::claim::language::external_product_claim(&normalized, &tokens)
                || crate::claim::language::live_runtime_claim(&normalized, &tokens)
        })
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
