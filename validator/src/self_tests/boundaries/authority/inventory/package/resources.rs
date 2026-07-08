use super::fixture_workspace::{
    assert_contains, assert_row, cleanup, failure_text, failures, inventory, temp_root,
    write_canonical_bin, write_file, write_manifest,
};

#[test]
fn unregistered_package_owned_source_fails() {
    let root = temp_root("source-orphan");
    write_canonical_bin(&root);
    write_file(&root, "validator/src/cli/orphan.rs", "pub fn run() {}\n");

    let failures = failures(&root, inventory(&["validator/src/bin/ultragoal.rs"]));
    assert_contains(
        &failures,
        "surface=source-orphan:validator/src/cli/orphan.rs",
    );
    cleanup(root);
}

#[test]
fn orphan_binary_source_without_cargo_owner_fails() {
    let root = temp_root("orphan-binary-source");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/bin/legacy_observer.rs",
        "fn main() {}\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/bin/legacy_observer.rs",
        ]),
    );
    assert_contains(
        &failures,
        "surface=binary-orphan:validator/src/bin/legacy_observer.rs",
    );
    assert_contains(
        &failures,
        "surface=binary-resource-orphan:validator/src/bin/legacy_observer.rs",
    );
    cleanup(root);
}

#[test]
fn unregistered_schema_or_fixture_surface_fails() {
    let root = temp_root("resource-orphan");
    write_canonical_bin(&root);
    write_file(&root, "schemas/claim-authority.schema.json", "{}\n");
    write_file(&root, "fixtures/red/claim-authority-red.json", "{}\n");

    let failures = failures(&root, inventory(&["validator/src/bin/ultragoal.rs"]));
    assert_contains(
        &failures,
        "surface=schema-orphan:schemas/claim-authority.schema.json",
    );
    assert_contains(
        &failures,
        "surface=fixture-orphan:fixtures/red/claim-authority-red.json",
    );
    cleanup(root);
}

#[test]
fn unregistered_generated_control_resources_fail_by_product_role() {
    let root = temp_root("generated-control-resource-orphans");
    write_canonical_bin(&root);
    write_file(&root, "templates/final-packet-blocker.json", "{}\n");
    write_file(&root, "templates/update-goal-blocker.json", "{}\n");

    let failures = failures(&root, inventory(&["validator/src/bin/ultragoal.rs"]));
    assert_contains(
        &failures,
        "surface=final_packet_blocker-orphan:templates/final-packet-blocker.json",
    );
    assert_contains(
        &failures,
        "surface=update_goal_blocker-orphan:templates/update-goal-blocker.json",
    );
    cleanup(root);
}

#[test]
fn inventoried_control_resources_receive_product_roles() {
    let root = temp_root("inventoried-control-resources");
    write_canonical_bin(&root);
    write_file(&root, "templates/final-packet-blocker.json", "{}\n");
    write_file(&root, "templates/update-goal-blocker.json", "{}\n");
    write_file(&root, "validation_artifacts/review/report.json", "{}\n");
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "templates/final-packet-blocker.json",
            "templates/update-goal-blocker.json",
            "validation_artifacts/review/report.json",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "templates/final-packet-blocker.json",
        "final_packet_blocker",
        "canonical",
    );
    assert_row(
        &inventory,
        "templates/update-goal-blocker.json",
        "update_goal_blocker",
        "canonical",
    );
    assert_row(
        &inventory,
        "validation_artifacts/review/report.json",
        "receipt_or_report",
        "external_debug_no_claim",
    );
    cleanup(root);
}

#[test]
fn dead_surface_preservation_name_fails() {
    let root = temp_root("coverage-wrapper");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/self_tests/coverage_only_wrapper.rs",
        "#[test]\nfn hit_count_wrapper() {}\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/self_tests/coverage_only_wrapper.rs",
        ]),
    );
    let text = failure_text(&failures);
    assert!(text.contains("coverage-only-wrapper"), "{failures:?}");
    assert!(text.contains("hit-count-wrapper"), "{failures:?}");
    cleanup(root);
}

#[test]
fn canonical_cli_source_with_registered_module_passes_package_surface_edges() {
    let root = temp_root("canonical-cli");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/source_topology.rs",
        "pub fn run() {}\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/source_topology.rs",
        ]),
    );
    assert!(
        !failure_text(&failures).contains("purpose_backed_surface_violation"),
        "{failures:?}"
    );
    cleanup(root);
}
