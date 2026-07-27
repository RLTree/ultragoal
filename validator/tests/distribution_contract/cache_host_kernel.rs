use crate::distribution::{
    CacheExpectation, DistributionErrorId as ErrorId, ExpectedPrior, HostCommandPlan, InstallPlan,
    InstallScope, PackageIdentity, SourceIdentity, reconcile_cache_read_only,
    reject_stale_version_reuse,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID};
use serde_json::json;

const A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const D: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn package(version: &str, tree: &str, archive: &str) -> PackageIdentity {
    PackageIdentity::new(
        SourceIdentity::new(
            CONTEXT_ID.into(),
            CANDIDATE_ID.into(),
            "harness-ultragoal".into(),
            version.into(),
            A.into(),
            A.into(),
        )
        .unwrap(),
        tree.into(),
        archive.into(),
    )
    .unwrap()
}

#[test]
fn cache_reconciliation_is_read_only_and_rejects_wrong_root_or_duplicate_versions() {
    let expected = CacheExpectation::new(
        CONTEXT_ID.into(),
        CANDIDATE_ID.into(),
        C.into(),
        "local-harness-plugins".into(),
        "harness-ultragoal".into(),
        "0.0.12".into(),
        B.into(),
    )
    .unwrap();
    let valid = json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":CONTEXT_ID,"candidate_id":CANDIDATE_ID,"cache_root_id":C,
        "entries":[{"marketplace":"local-harness-plugins","plugin_id":"harness-ultragoal","version":"0.0.12","package_tree_sha256":B}]
    });
    let bytes = serde_json::to_vec(&valid).unwrap();
    let before = bytes.clone();
    assert_eq!(
        reconcile_cache_read_only(&bytes, &expected)
            .unwrap()
            .package_tree_sha256(),
        B
    );
    assert_eq!(bytes, before);
    let mut wrong_root = valid.clone();
    wrong_root["cache_root_id"] = json!(D);
    assert_eq!(
        reconcile_cache_read_only(&serde_json::to_vec(&wrong_root).unwrap(), &expected)
            .unwrap_err()
            .id(),
        ErrorId::InvalidSpec
    );
    for (marketplace, version) in [
        ("local-harness-plugins", "0.0.11"),
        ("other-marketplace", "0.0.12"),
    ] {
        let mut duplicate = valid.clone();
        duplicate["entries"].as_array_mut().unwrap().push(json!({
            "marketplace":marketplace,"plugin_id":"harness-ultragoal",
            "version":version,"package_tree_sha256":D
        }));
        assert_eq!(
            reconcile_cache_read_only(&serde_json::to_vec(&duplicate).unwrap(), &expected)
                .unwrap_err()
                .id(),
            ErrorId::InstallConflict
        );
    }
}

#[test]
fn host_command_plans_use_exact_argv_and_explicit_rollback_plans() {
    let root = "/tmp/repo with spaces;touch SHOULD_NOT_RUN";
    let identity = package("0.0.12", B, C);
    let plan =
        HostCommandPlan::repository_install(&identity, root, "harness-ultragoal-repo").unwrap();
    assert!(!format!("{plan:?}").contains(root));
    assert_eq!(plan.commands()[0].program(), "codex");
    assert!(plan.commands().iter().all(|command| {
        command.environment().is_empty()
            && command.timeout_ms() == 30_000
            && command.max_attempts() == 1
    }));
    assert_eq!(
        plan.commands()[0].argv(),
        ["plugin", "marketplace", "add", root]
    );
    assert!(HostCommandPlan::repository_install(&identity, "/tmp/repo\nattack", "repo").is_err());
    assert_eq!(
        HostCommandPlan::personal_install(&identity, "local-harness-plugins")
            .unwrap()
            .commands()[0]
            .argv(),
        ["plugin", "add", "harness-ultragoal@local-harness-plugins"]
    );
    assert_eq!(
        HostCommandPlan::personal_remove(&identity, "local-harness-plugins")
            .unwrap()
            .commands()[0]
            .argv(),
        [
            "plugin",
            "remove",
            "harness-ultragoal@local-harness-plugins"
        ]
    );
    assert_eq!(
        HostCommandPlan::repository_remove(&identity, "harness-ultragoal-repo")
            .unwrap()
            .commands()[1]
            .argv(),
        ["plugin", "marketplace", "remove", "harness-ultragoal-repo"]
    );
}

#[test]
fn stale_version_reuse_downgrades_and_wrong_install_roots_are_rejected() {
    let current = package("0.0.12", B, C);
    assert_eq!(
        reject_stale_version_reuse(&current, &package("0.0.12", D, C))
            .unwrap_err()
            .id(),
        ErrorId::InstallConflict
    );
    assert_eq!(
        reject_stale_version_reuse(&current, &package("0.0.11", B, C))
            .unwrap_err()
            .id(),
        ErrorId::InstallConflict
    );
    reject_stale_version_reuse(&current, &package("0.0.13", D, A)).unwrap();
    for scope in [InstallScope::Repository, InstallScope::Personal] {
        assert!(
            InstallPlan::new(
                CONTEXT_ID.into(),
                CANDIDATE_ID.into(),
                scope,
                "plugins/harness-ultragoal.hugpkg".into(),
                C.into(),
                ExpectedPrior::Absent
            )
            .is_ok()
        );
        assert!(
            InstallPlan::new(
                CONTEXT_ID.into(),
                CANDIDATE_ID.into(),
                scope,
                "wrong/root.hugpkg".into(),
                C.into(),
                ExpectedPrior::Absent
            )
            .is_err()
        );
    }
}
