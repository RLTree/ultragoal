pub(crate) fn review_round_errors(
    root: &Path,
    receipt: &Value,
    anchors: &AnchorValues,
    out: &mut Vec<ReviewFailure>,
) {
    let gate = &receipt["materiality_gate"];
    for error in failures(gate, Some(root), Some(receipt), Some(anchors)) {
        out.push(ReviewFailure::new(
            "material-review-scope-gate",
            &error,
            "materiality_gate",
        ));
    }
    if receipt.get("review_stage").and_then(Value::as_str) == Some("falsification") {
        if text(gate, "decision") != FULL {
            out.push(failure("material_falsification_requires_full_scope"));
        }
        if gate["output_may_be_used_for_material_signoff"].as_bool() != Some(false) {
            out.push(failure("materiality_reviewer_cannot_authorize_signoff"));
        }
    }
}

pub(crate) fn fixture_failures(root: &Path) -> Vec<String> {
    let dir = root.join("fixtures/review-materiality/valid");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec!["review_materiality_fixture_dir_missing".to_string()];
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json")).then_some(path)
        })
        .flat_map(|path| match crate::json_boundary::read_json(&path) {
            Ok(value) => failures(&value, Some(root), None, None)
                .into_iter()
                .map(move |error| format!("{}: {error}", path.display()))
                .collect::<Vec<_>>(),
            Err(err) => vec![format!("{}: {err}", path.display())],
        })
        .collect()
}

pub(crate) fn value_failures(value: &Value) -> Vec<String> {
    failures(value, None, None, None)
}

fn failures(
    value: &Value,
    root: Option<&Path>,
    receipt: Option<&Value>,
    bound_anchors: Option<&AnchorValues>,
) -> Vec<String> {
    let mut out = Vec::new();
    if text(value, "schema") != "harness-ultragoal.review-materiality-gate.v2" {
        out.push("materiality_schema_invalid".into());
    }
    if text(value, "decision_authority") != "deterministic_rust"
        || text(value, "reviewer_output_authority") != "falsification_only_cannot_raise_claims"
    {
        out.push("materiality_authority_invalid".into());
    }
    let source = receipt
        .map(policy::receipt_source)
        .unwrap_or(Some(AnchorSource::StaticFixture));
    let anchors_ok = identity_errors(value, root, receipt, source, bound_anchors, &mut out);
    let authority = match source {
        Some(AnchorSource::StaticFixture) => MaterialityAuthority::StaticFixture,
        Some(AnchorSource::Live) => MaterialityAuthority::LiveObservationUnavailable,
        None => MaterialityAuthority::CanonicalSourceUnavailable,
    };
    let derived = config::derive_materiality(value, anchors_ok, authority);
    out.extend(derived.errors);
    let decision = text(value, "decision");
    if ![FULL, DELTA, ADVISORY, BLOCKED].contains(&decision) {
        out.push("materiality_decision_invalid".into());
    } else if decision != derived.decision {
        out.push("materiality_decision_derivation_mismatch".into());
    }
    claim_ceiling_errors(value, &mut out);
    match decision {
        FULL => full_errors(value, &mut out),
        DELTA => delta_errors(value, &mut out),
        ADVISORY => advisory_errors(value, &mut out),
        BLOCKED => blocked_errors(value, &mut out),
        _ => {}
    }
    out
}

fn identity_errors(
    value: &Value,
    root: Option<&Path>,
    receipt: Option<&Value>,
    source: Option<AnchorSource>,
    bound_anchors: Option<&AnchorValues>,
    out: &mut Vec<String>,
) -> bool {
    let expected_source = source.unwrap_or(AnchorSource::StaticFixture);
    let expected = policy::expected_anchors(expected_source);
    let anchors = match bound_anchors {
        Some(bound) => crate::review::round::anchor::refs::verify_loaded(
            value,
            "anchors_checked",
            expected,
            &bound.materiality_anchors,
        ),
        None => {
            crate::review::round::anchor::refs::verify(value, "anchors_checked", expected, root)
        }
    };
    let registry = crate::review::round::anchor::refs::verify(
        value,
        "reviewer_registry_evidence",
        policy::expected_registry(expected_source),
        root,
    );
    if anchors.is_none() {
        if value["anchors_checked"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            out.push("materiality_anchor_evidence_missing".into());
        }
        out.push("materiality_anchor_identity_invalid".into());
    }
    if registry.is_none() {
        if value["reviewer_registry_evidence"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            out.push("materiality_reviewer_registry_evidence_missing".into());
        }
        out.push("materiality_reviewer_registry_identity_invalid".into());
    }
    let contents_ok = root.is_some()
        && anchors
            .as_ref()
            .is_some_and(|rows| anchor_contents_valid(rows))
        && registry
            .as_ref()
            .is_some_and(|rows| registry_contents_valid(&rows[0]));
    if !contents_ok {
        out.push("materiality_gate_evidence_unverified".into());
    }
    let linked = receipt.is_none_or(|receipt| {
        let paths = [
            text_at(receipt, "/validator_receipt/path"),
            text_at(receipt, "/review_target/path"),
            text_at(receipt, "/archive/path"),
        ];
        anchors.as_ref().is_some_and(|rows| {
            paths
                .iter()
                .all(|path| rows.iter().any(|row| row.path == *path))
        }) && registry.as_ref().is_some_and(|rows| {
            rows[0].path == text_at(receipt, "/live_registry_exposure/path")
                && rows[0].digest == text_at(receipt, "/live_registry_exposure/digest")
        })
    });
    if !linked {
        out.push("materiality_evidence_substitution".into());
    }
    anchors.is_some() && registry.is_some() && contents_ok && linked
}

fn anchor_contents_valid(rows: &[crate::review::round::anchor::refs::VerifiedRef]) -> bool {
    rows.iter().all(|row| {
        row.value
            .as_ref()
            .is_some_and(|value| value["status"] == "pass")
    })
}

fn claim_ceiling_errors(value: &Value, out: &mut Vec<String>) {
    let Some(ceiling) = value.get("claim_ceiling") else {
        out.push("materiality_claim_ceiling_missing".into());
        return;
    };
    if ["supported", "unsupported", "blocked"]
        .iter()
        .any(|key| !ceiling.get(key).is_some_and(Value::is_array))
    {
        out.push("materiality_claim_ceiling_missing".into());
    }
}
