use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn canonical_path_preserves_exact_ascii_case_without_expanding_authority() {
    let original = "Validator/Source/MixedCase.RS";
    let parsed = CanonicalPath::parse(original).unwrap();
    assert_eq!(parsed.as_str(), original);

    let encoded = serde_json::to_vec(&parsed).unwrap();
    let decoded: CanonicalPath = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.as_str(), original);
}

#[test]
fn host_protected_ascii_case_aliases_fail_closed_in_every_path_category() {
    let aliases = [
        ".GIT/config",
        ".GiT/config",
        ".CODEX/agents/writer.toml",
        ".AgEnTs/writer.toml",
        ".CODEX-WORKTREE/ENV.SH",
    ];

    for alias in aliases {
        let protected = path(alias);
        let mut read_policy = policy();
        read_policy.allowed_read_paths.insert(protected.clone());
        let mut encoded = serde_json::to_value(lease()).unwrap();
        encoded["read_paths"] = serde_json::json!([alias]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        let error = substituted.validate(&read_policy).unwrap_err();
        assert_eq!(error, OrchestrationError::RootOnlyScope);
        assert!(!error.to_string().contains(alias));

        for (location, category) in [
            ("/owned_scope/paths", 0),
            ("/owned_scope/generated_outputs", 1),
            ("/owned_scope/fixtures", 2),
        ] {
            let mut write_policy = policy();
            match category {
                0 => write_policy.allowed_paths.insert(protected.clone()),
                1 => write_policy
                    .allowed_generated_outputs
                    .insert(protected.clone()),
                _ => write_policy.allowed_fixtures.insert(protected.clone()),
            };
            let mut encoded = serde_json::to_value(lease()).unwrap();
            *encoded.pointer_mut(location).unwrap() = serde_json::json!([alias]);
            let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
            let error = substituted.validate(&write_policy).unwrap_err();
            assert_eq!(error, OrchestrationError::RootOnlyScope);
            assert!(!error.to_string().contains(alias));
        }
    }
}

#[test]
fn observed_case_aliases_are_denied_when_the_host_exposes_them() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("n10-case-alias-observation-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(root.join(".git")).unwrap();

    if root.join(".GIT").is_dir() {
        let protected = path(".GIT/config");
        let mut expanded = policy();
        expanded.allowed_paths.insert(protected.clone());
        let mut owned = scope("node_a");
        owned.paths = [protected].into();
        assert_eq!(
            lease_with_scope("lease-001", "node-a", "worker-a", owned)
                .validate(&expanded)
                .unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generic_root_only_ascii_case_aliases_are_denied_in_every_write_path_category() {
    for category in 0..3 {
        let protected_alias = path("PLUGIN-MANIFEST-DRAFT.JSON");
        let mut expanded = policy();
        let location = match category {
            0 => {
                expanded.allowed_paths.insert(protected_alias.clone());
                "/owned_scope/paths"
            }
            1 => {
                expanded
                    .allowed_generated_outputs
                    .insert(protected_alias.clone());
                "/owned_scope/generated_outputs"
            }
            _ => {
                expanded.allowed_fixtures.insert(protected_alias.clone());
                "/owned_scope/fixtures"
            }
        };
        let mut encoded = serde_json::to_value(lease()).unwrap();
        *encoded.pointer_mut(location).unwrap() = serde_json::json!([protected_alias.as_str()]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            substituted.validate(&expanded).unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
    }
}

#[test]
fn allowed_path_prefixes_remain_exact_case_authority() {
    for (location, category, alias) in [
        ("/read_paths", 0, "DOCS/ultragoal-successor-live"),
        (
            "/owned_scope/paths",
            1,
            "VALIDATOR/src/orchestration/node_a.rs",
        ),
        (
            "/owned_scope/generated_outputs",
            2,
            "GENERATED/orchestration/node_a.json",
        ),
        (
            "/owned_scope/fixtures",
            3,
            "VALIDATOR/tests/orchestration_contract/node_a.json",
        ),
    ] {
        let mut encoded = serde_json::to_value(lease()).unwrap();
        *encoded.pointer_mut(location).unwrap() = serde_json::json!([alias]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            substituted.validate(&policy()).unwrap_err(),
            OrchestrationError::UnknownScope,
            "case alias in category {category} must not gain authority"
        );
    }
}

#[test]
fn owned_scope_rejects_ascii_case_alias_duplicates_across_categories() {
    let mut duplicated = scope("node_a");
    duplicated.paths = [path("validator/src/orchestration/node_a.rs")].into();
    duplicated.generated_outputs = [path("VALIDATOR/SRC/ORCHESTRATION/NODE_A.RS")].into();
    assert_eq!(
        duplicated.validate().unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn two_leases_cannot_split_ascii_case_aliases() {
    let mut registry = LeaseRegistry::default();
    registry.grant(lease(), &policy(), &binding()).unwrap();

    let alias = path("VALIDATOR/SRC/ORCHESTRATION/NODE_A.RS");
    let mut other = scope("node_b");
    other.paths = [alias.clone()].into();
    let mut expanded = policy();
    expanded.allowed_paths.insert(alias);

    assert_eq!(
        registry
            .grant(
                lease_with_scope("lease-002", "node-b", "worker-b", other),
                &expanded,
                &binding(),
            )
            .unwrap_err(),
        OrchestrationError::LeaseConflict
    );
}

#[test]
fn semantic_root_only_overlap_is_bidirectional_and_segment_bounded() {
    let mut expanded = policy();
    expanded
        .root_only_semantic_prefixes
        .insert("orchestration::claims".to_owned());

    for requested in [
        "orchestration",
        "orchestration::claims",
        "orchestration::claims::child",
    ] {
        let mut encoded = serde_json::to_value(lease()).unwrap();
        encoded["owned_scope"]["semantic_symbols"] = serde_json::json!([requested]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        let error = substituted.validate(&expanded).unwrap_err();
        assert_eq!(error, OrchestrationError::RootOnlyScope);
        assert!(!error.to_string().contains(requested));
    }

    for sibling in ["orchestration::claimsmith", "orchestration::other"] {
        let mut encoded = serde_json::to_value(lease()).unwrap();
        encoded["owned_scope"]["semantic_symbols"] = serde_json::json!([sibling]);
        let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
        substituted.validate(&expanded).unwrap();
    }

    let canary = "orchestration::SECRET_SEMANTIC_CANARY";
    expanded
        .root_only_semantic_prefixes
        .insert(format!("{canary}::private"));
    let mut encoded = serde_json::to_value(lease()).unwrap();
    encoded["owned_scope"]["semantic_symbols"] = serde_json::json!([canary]);
    let substituted: LeaseSpec = serde_json::from_value(encoded).unwrap();
    let error = substituted.validate(&expanded).unwrap_err();
    assert_eq!(error, OrchestrationError::RootOnlyScope);
    assert!(!error.to_string().contains("SECRET_SEMANTIC_CANARY"));
}
