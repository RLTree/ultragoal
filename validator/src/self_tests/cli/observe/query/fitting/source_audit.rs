const REVIEW_ROUND_PROOF: &str =
    "validation_artifacts/observability/review-round-verify-production-proof.json";
const REVIEW_ROUND_EXPLAIN: &str =
    "validation_artifacts/observability/review-round-verify-explain-failure.json";

#[test]
fn source_audit_inventory_records_stable_production_proof() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/source audit")
        .expect("source audit row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert!(
        row["receipt_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/source-audit-production-proof.json")),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/source-audit-explain-failure.json")),
        "{row}"
    );
    let board = inventory
        .pointer("/fitting_control_board/first_incomplete")
        .expect("first incomplete");
    assert_eq!(board["id"], "install audit");
    assert_eq!(board["fitting_status"], "partially_fitted");
    assert_eq!(
        board["next_unfitted_surface"],
        "red/green/tamper fixture proof"
    );
    let review_round = inventory
        .pointer("/fitting_inventory/review-round verify")
        .expect("review-round row");
    assert_eq!(review_round["fitting_status"], "fitted");
    assert_eq!(review_round["next_unfitted_surface"], "none");
    assert_eq!(review_round["missing_surfaces"], serde_json::json!([]));
    for (key, path) in [
        ("receipt_paths", REVIEW_ROUND_PROOF),
        ("same_candidate_query_proof_paths", REVIEW_ROUND_EXPLAIN),
    ] {
        assert!(
            review_round[key]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item.as_str() == Some(path)),
            "{key}: {review_round}"
        );
    }
    for key in ["red_fixtures", "green_fixtures", "tamper_fixtures"] {
        assert!(
            review_round[key]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{key}: {review_round}"
        );
    }
}

#[test]
fn fit_repo_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/fit-repo prove")
        .expect("fit repo row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert!(
        row["receipt_paths"].as_array().unwrap().iter().any(|item| {
            item.as_str().is_some_and(|text| {
                text == "validation_artifacts/observability/fit-repo-prove-production-proof.json"
            })
        }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str().is_some_and(|text| {
                    text == "validation_artifacts/observability/fit-repo-prove-explain-failure.json"
                })
            }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item
                .as_str()
                .is_some_and(|text| !text.ends_with("-production-proof.json"))),
        "{row}"
    );
    for key in ["red_fixtures", "green_fixtures", "tamper_fixtures"] {
        assert!(
            row[key].as_array().is_some_and(|items| !items.is_empty()),
            "{key}: {row}"
        );
    }
}

#[test]
fn product_journey_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/product prove-journey")
        .expect("product journey row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert!(
        row["receipt_paths"].as_array().unwrap().iter().any(|item| {
            item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/product-prove-journey-production-proof.json")
        }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/product-prove-journey-explain-failure.json")
            }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item
                .as_str()
                .is_some_and(|text| !text.ends_with("-production-proof.json"))),
        "{row}"
    );
    for key in ["red_fixtures", "green_fixtures", "tamper_fixtures"] {
        assert!(
            row[key].as_array().is_some_and(|items| !items.is_empty()),
            "{key}: {row}"
        );
    }
}

#[test]
fn product_cohesion_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/product prove-cohesion")
        .expect("product cohesion row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert!(
        row["receipt_paths"].as_array().unwrap().iter().any(|item| {
            item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/product-prove-cohesion-production-proof.json")
        }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/product-prove-cohesion-explain-failure.json")
            }),
        "{row}"
    );
    assert!(
        row["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item
                .as_str()
                .is_some_and(|text| !text.ends_with("-production-proof.json"))),
        "{row}"
    );
    for key in ["red_fixtures", "green_fixtures", "tamper_fixtures"] {
        assert!(
            row[key].as_array().is_some_and(|items| !items.is_empty()),
            "{key}: {row}"
        );
    }
}

#[test]
fn product_fitness_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/product prove-fitness")
        .expect("product fitness row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert!(
        row["receipt_paths"].as_array().unwrap().iter().any(|item| {
            item.as_str().is_some_and(|text| text
                == "validation_artifacts/observability/product-prove-fitness-production-proof.json")
        }),
        "{row}"
    );
    for key in ["red_fixtures", "green_fixtures", "tamper_fixtures"] {
        assert!(
            row[key].as_array().is_some_and(|items| !items.is_empty()),
            "{key}: {row}"
        );
    }
}
