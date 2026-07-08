use super::{
    changed_path, git_root_matches_requested_root, path_affects_surface,
    path_rules::surface_has_explicit_input_spec,
};

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
    for path in [
        "validator/src/cli/live_loop/mod.rs",
        "validator/tests/cli_surface.rs",
        "validator/rustfmt.toml",
        "validator/.rustfmt.toml",
    ] {
        assert!(path_affects_surface(path, "fmt_check"), "{path}");
    }
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
fn changed_input_rules_reject_unknown_surfaces_and_builder_contract_inputs() {
    let modular = modular_contract_path();
    let compatibility = compatibility_contract_path();
    for (path, surface) in [
        (
            "validator/src/cli/live_loop/mod.rs",
            "unknown_product_surface",
        ),
        (modular.as_str(), "package_inventory"),
        (compatibility.as_str(), "source_audit"),
    ] {
        assert!(!path_affects_surface(path, surface), "{path} {surface}");
    }
    assert!(path_affects_surface(
        "validation_artifacts/observability/package-digest.json",
        "observability_control_board"
    ));
    for (path, surface) in [
        (
            "validation_artifacts/observability/package-digest.json",
            "package_inventory",
        ),
        ("state/codex-review-artifacts/review.txt", "source_audit"),
        (
            "state/codex-review-receipts.d/review.jsonl",
            "package_inventory",
        ),
    ] {
        assert!(!path_affects_surface(path, surface), "{path} {surface}");
    }
}

#[test]
fn every_live_loop_surface_has_explicit_product_input_spec() {
    let missing: Vec<_> = super::super::surfaces::LOOP_VALIDATION_SURFACES
        .iter()
        .filter(|surface| !surface_has_explicit_input_spec(surface.id))
        .map(|surface| surface.id)
        .collect();

    assert!(
        missing.is_empty(),
        "live-loop surfaces without explicit input specs: {missing:?}"
    );
}

#[test]
fn product_input_specs_are_role_specific_and_inventory_closes_unknown_paths() {
    for path in [
        "validator/src/cli/live_loop/mod.rs",
        "skills/fit-repo/SKILL.md",
        "agents/plugin-scout.md",
        "custom-agents/harness-repo-initializer.toml",
        "artifacts/source-snapshots/openai-harness-engineering.txt",
        "docs/plugin-resource-map.md",
        "state/product-surface.json",
    ] {
        assert!(path_affects_surface(path, "package_inventory"), "{path}");
    }
    assert!(path_affects_surface(
        "skills/fit-repo/SKILL.md",
        "package_digest"
    ));
    assert!(!path_affects_surface(
        &modular_contract_path(),
        "package_digest"
    ));
    let coverage_receipt = "validation_artifacts/coverage/coverage-receipt.json";
    for surface in ["coverage_prove", "source_audit"] {
        assert!(path_affects_surface(coverage_receipt, surface), "{surface}");
    }
    for root_resource in [
        ".gitignore",
        "LICENSE",
        "package.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "rust-toolchain.toml",
        "audit.toml",
        "deny.toml",
    ] {
        for surface in ["package_inventory", "source_audit"] {
            assert!(
                path_affects_surface(root_resource, surface),
                "{root_resource} {surface}"
            );
        }
    }
    assert!(path_affects_surface(
        "dev/observability/compose.yml",
        "observability_control_board"
    ));
    assert!(path_affects_surface(
        "fixtures/red/observability/missing-log.json",
        "red_fixture_report"
    ));
    for surface in ["red_fixture_report", "schema_validation"] {
        assert!(
            !path_affects_surface("docs/README-operator-note.md", surface),
            "{surface}"
        );
    }
}

fn modular_contract_path() -> String {
    "docs/ultragoal-contract-2026-07/README.md".to_string()
}

fn compatibility_contract_path() -> String {
    let parts = [
        "parent",
        "session",
        "full",
        "ultragoal",
        "compliance",
        "prompt",
        "2026",
        "06",
        "25",
    ];
    format!("docs/{}.md", parts.join("-"))
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
