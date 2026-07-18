#[test]
fn dep_info_and_policy_reject_outside_root_and_parent_paths() {
    use super::plugin_product::source_closure::{
        BuildClosurePolicy, BuildInputKind, ClosureError, RequiredBuildInput,
    };
    let root = ClosureTempRoot::new();
    let outside = std::env::temp_dir().join(format!("hul-outside-{}.rs", std::process::id()));
    std::fs::write(&outside, b"outside\n").unwrap();
    std::fs::write(
        root.path().join("target/outside.d"),
        format!("target/probe: {}\n", outside.display()),
    )
    .unwrap();
    assert_eq!(
        BuildClosurePolicy::from_dep_info(
            root.path(),
            &["target/outside.d".to_owned()],
            Vec::new(),
        ),
        Err(ClosureError::OutsideRoot)
    );
    let _ = std::fs::remove_file(outside);
    assert_eq!(
        BuildClosurePolicy::new(vec![RequiredBuildInput {
            path: "../escape".to_owned(),
            kind: BuildInputKind::DynamicInput,
        }]),
        Err(ClosureError::InvalidPath)
    );
}

#[test]
fn live_plugin_product_build_closure_is_exact_and_byte_identical_twice() {
    use super::plugin_product::source_closure::{BuildClosureV1, plugin_product_build_policy};
    let policy = plugin_product_build_policy().unwrap();
    let first = BuildClosureV1::capture(&root(), &policy).unwrap();
    let second = BuildClosureV1::capture(&root(), &policy).unwrap();
    if std::env::var_os("HUL_PRINT_BUILD_CLOSURE").is_some() {
        println!("{}", serde_json::to_string(&first).unwrap());
    }
    assert_eq!(first, second);
    assert_eq!(first.rows.len(), policy.required_inputs.len());
    assert_eq!(first.rows.len(), 122);
    assert_eq!(
        first
            .rows
            .iter()
            .filter(|row| row
                .path
                .starts_with("validator/src/plugin_product/host_lifecycle/"))
            .count(),
        51,
        "the supported-host transaction source must remain in the candidate closure"
    );
    first.verify(&root(), &policy).unwrap();
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildClosureSummary {
    schema_version: String,
    aggregate_sha256: String,
    row_count: usize,
    final_session_revalidated: bool,
    policy: String,
    root_extension_required: bool,
}

#[test]
fn superseded_lifecycle_worker_result_is_typed_but_stale_after_authorization_correction() {
    use super::plugin_product::product_fitness::{
        ClaimCeiling, DimensionDisposition, OverallDisposition, ProductFitnessDisposition,
    };
    use super::plugin_product::source_closure::{BuildClosureV1, plugin_product_build_policy};

    let envelope: LifecycleRootEnvelope =
        serde_json::from_str(&read(LIFECYCLE_ENVELOPE_PATH)).unwrap();
    let result =
        WorkerResultV1::parse_json(&std::fs::read(root().join(LIFECYCLE_RESULT_PATH)).unwrap())
            .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "superseded lifecycle result must not verify corrected source bytes"
    );
    assert_eq!(
        result.base_state["root_envelope_no_claim_statement"],
        envelope.no_claim_statement
    );

    let fitness: ProductFitnessDisposition =
        serde_json::from_value(result.final_state["product_fitness_disposition"].clone()).unwrap();
    assert_eq!(fitness.overall, OverallDisposition::Blocked);
    assert!(
        fitness
            .dimensions
            .iter()
            .all(|dimension| dimension.disposition == DimensionDisposition::Blocked)
    );
    assert!(
        fitness
            .truth_layer_ceilings
            .values()
            .all(|ceiling| *ceiling == ClaimCeiling::Withheld)
    );

    let stale_summary: BuildClosureSummary =
        serde_json::from_value(result.final_state["build_closure"].clone()).unwrap();
    let current =
        BuildClosureV1::capture(&root(), &plugin_product_build_policy().unwrap()).unwrap();
    assert_ne!(stale_summary.aggregate_sha256, current.aggregate_sha256);
    assert_eq!(stale_summary.schema_version, current.schema_version);
    assert_eq!(stale_summary.row_count, current.rows.len());
    assert!(stale_summary.final_session_revalidated);
    assert_eq!(stale_summary.policy, "plugin_product_build_policy");
    assert!(stale_summary.root_extension_required);
}

#[test]
fn authorization_correction_worker_result_is_typed_but_stale_after_recovery_activation() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_AUTH_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_AUTH_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_AUTH_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "authorization correction must stale when recovery source and tests change"
    );
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_AUTH_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
}
