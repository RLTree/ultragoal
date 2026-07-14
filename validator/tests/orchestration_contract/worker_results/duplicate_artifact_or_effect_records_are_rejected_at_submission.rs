#[test]
fn duplicate_artifact_or_effect_records_are_rejected_at_submission() {
    let (lease, package, mut result) = subject();
    result.artifacts.push(result.artifacts[0].clone());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );

    let (_, _, mut result) = subject();
    result.effects.push(result.effects[0].clone());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn dependency_set_must_match_work_package() {
    let (lease, mut package, result) = subject();
    package.dependencies.insert("upstream".to_owned());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn worker_result_may_report_unresolved_dependency_without_self_accepting() {
    let (lease, package, mut result) = subject();
    result.unresolved_dependencies = vec!["root-wiring".to_owned()];
    result.validate_for(&lease, &package).unwrap();
}

#[test]
fn broad_owned_child_does_not_authorize_touching_its_parent() {
    let mut lease = lease();
    lease.owned_scope.paths = [path("validator/src/orchestration/node_a")].into();
    let mut package = package("node-a", &[], "node_a");
    package.owned_scope = lease.owned_scope.clone();
    let mut result = result_for(&lease, &package);
    result.touched_paths = vec!["validator/src/orchestration".to_owned()];
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::UnknownScope
    );
}

#[test]
fn root_change_requests_are_machine_exact_unknown_field_closed_and_alias_unique() {
    let (lease, package, result) = subject();
    for field in ["path", "expected_sha256"] {
        let mut value = serde_json::to_value(&result).unwrap();
        value["requested_root_changes"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(WorkerResultV1::parse_json(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut value = serde_json::to_value(&result).unwrap();
    value["requested_root_changes"][0]["alternate_path"] = json!("other.json");
    assert!(WorkerResultV1::parse_json(&serde_json::to_vec(&value).unwrap()).is_err());

    let mut invalid = result.clone();
    invalid.requested_root_changes[0].expected_sha256 = "bad".to_owned();
    assert_eq!(
        invalid.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidDigest
    );
    for alias in [
        "validator/src/lib.rs",
        "VALIDATOR/SRC/LIB.RS",
        "validator/src",
    ] {
        let mut duplicated = result.clone();
        duplicated.requested_root_changes.push(
            RootChangeRequest::new(alias, &digest('7'), Some("ambiguous second request")).unwrap(),
        );
        assert_eq!(
            duplicated.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::DuplicateOutput
        );
    }
    assert!(RootChangeRequest::new("other.json", &digest('7'), Some("bad\ntext")).is_err());
}

#[test]
fn reviewed_root_change_order_cannot_be_substituted() {
    let (lease, package, mut result) = subject();
    result
        .requested_root_changes
        .push(RootChangeRequest::new("plugin-manifest-draft.json", &digest('7'), None).unwrap());
    let review = review_for_result(&result, ReviewDecision::Pass);
    let original_id = result.result_id().unwrap();
    result.requested_root_changes.reverse();
    assert_ne!(result.result_id().unwrap(), original_id);
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::InvalidReview
    );
}
