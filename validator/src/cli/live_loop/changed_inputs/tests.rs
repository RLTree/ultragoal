use super::{changed_path, git_root_matches_requested_root, path_affects_surface};

#[test]
fn changed_path_extracts_status_and_rename_target() {
    assert_eq!(changed_path("   "), None);
    assert_eq!(
        changed_path(" M validator/src/cli/live_loop/mod.rs").as_deref(),
        Some("validator/src/cli/live_loop/mod.rs")
    );
    assert_eq!(
        changed_path("R  old.rs -> validator/src/lib.rs").as_deref(),
        Some("validator/src/lib.rs")
    );
}

#[test]
fn changed_file_detection_rejects_parent_repo_leakage_for_temp_roots() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-git-root");
    std::fs::create_dir_all(&root).expect("root");
    assert!(!git_root_matches_requested_root(&root));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn changed_inputs_are_surface_local_for_hot_repair_nodes() {
    assert!(path_affects_surface(
        "validator/src/cli/live_loop/mod.rs",
        "fmt_check"
    ));
    assert!(!path_affects_surface("schemas/example.json", "fmt_check"));
    assert!(path_affects_surface(
        "schemas/example.json",
        "schema_validation"
    ));
    assert!(path_affects_surface(
        "docs/namespace-law-exceptions.json",
        "namespace_check"
    ));
    assert!(!path_affects_surface(
        "dev/observability/compose.yml",
        "build_check"
    ));
}

#[test]
fn changed_summary_keeps_changed_inputs_and_surface_selection_visible() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-affected-summary");
    std::fs::create_dir_all(root.join("validator/src/cli/live_loop")).expect("dirs");
    std::fs::write(
        root.join("validator/src/cli/live_loop/mod.rs"),
        "fn main() {}\n",
    )
    .expect("rust source");
    let inputs = super::ChangedInputs {
        changed_files_digest: "sha256:changed".to_string(),
        changed_file_count: 1,
        audit_context_digest: "sha256:context".to_string(),
        changed_files: vec!["validator/src/cli/live_loop/mod.rs".to_string()],
        surface_digests: super::super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .map(|surface| {
                (
                    surface.id,
                    super::surface_changed_digest(
                        &root,
                        *surface,
                        "sha256:candidate",
                        &["validator/src/cli/live_loop/mod.rs".to_string()],
                    ),
                )
            })
            .collect(),
        affected_surfaces: super::super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .map(|surface| {
                (
                    surface.id,
                    super::surface_is_affected(
                        *surface,
                        &["validator/src/cli/live_loop/mod.rs".to_string()],
                    ),
                )
            })
            .collect(),
    };
    let summary = inputs.summary();
    assert_eq!(summary["changed_file_count"], 1);
    assert!(
        summary["affected_nodes"]
            .as_array()
            .expect("affected nodes")
            .iter()
            .any(|node| node["node_id"] == "fmt_check")
    );
    assert!(
        summary["unaffected_nodes"]
            .as_array()
            .expect("unaffected nodes")
            .iter()
            .any(|node| node["node_id"] == "schema_validation")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn missing_changed_file_digest_is_stable_and_agent_legible() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-missing-file-digest",
    );
    std::fs::create_dir_all(&root).expect("root");

    assert_eq!(
        super::file_digest(&root, "validator/src/missing.rs"),
        crate::digest::bytes("missing:validator/src/missing.rs".as_bytes())
    );

    std::fs::remove_dir_all(root).expect("cleanup");
}
