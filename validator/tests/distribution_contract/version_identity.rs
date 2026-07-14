use crate::distribution::{
    CacheExpectation, DistributionErrorId, HostCommandPlan, Layer, MarketplaceExpectation,
    MarketplaceScope, PackageIdentity, SourceIdentity, reject_stale_version_reuse, verify,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture};

const A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn source(version: &str) -> Result<SourceIdentity, DistributionErrorId> {
    SourceIdentity::new(
        CONTEXT_ID.into(),
        CANDIDATE_ID.into(),
        "harness-ultragoal".into(),
        version.into(),
        A.into(),
        A.into(),
    )
    .map_err(|error| error.id())
}

fn package(version: &str, tree: &str, archive: &str) -> PackageIdentity {
    PackageIdentity::new(source(version).unwrap(), tree.into(), archive.into()).unwrap()
}

fn rejects(previous: &str, next: &str) {
    assert_eq!(
        reject_stale_version_reuse(&package(previous, A, B), &package(next, B, C),)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict,
        "{previous} -> {next}"
    );
}

fn allows(previous: &str, next: &str) {
    reject_stale_version_reuse(&package(previous, A, B), &package(next, B, C))
        .unwrap_or_else(|_| panic!("{previous} -> {next}"));
}

#[test]
fn source_identity_uses_the_canonical_manifest_version_grammar() {
    for valid in ["0.0.0", "1.2.3-alpha.1", "1.2.3-beta.2+codex.local-01"] {
        assert!(source(valid).is_ok(), "{valid}");
    }
    let oversized = format!("1.2.3+{}", "a".repeat(129));
    for invalid in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.3-01",
        "1.2.3-",
        "1.2.3+",
        "1.2.3+a..b",
        "1.2.3+a+b",
        "1.2.3-alpha..1",
        "1.2.3-α",
        "1.2.3\n",
        &oversized,
    ] {
        assert_eq!(source(invalid), Err(DistributionErrorId::InvalidSpec));
    }
}

#[test]
fn downgrade_rejection_uses_full_semver_precedence() {
    rejects("1.0.0", "1.0.0-rc.1");
    rejects("1.0.0-beta", "1.0.0-alpha.999");
    rejects("1.0.0-alpha.beta", "1.0.0-alpha.999");
    rejects("1.0.0-alpha.10", "1.0.0-alpha.2");
    allows("1.0.0-alpha", "1.0.0-alpha.1");
    allows("1.0.0-alpha.2", "1.0.0-alpha.10");
    allows("1.0.0-alpha.999", "1.0.0-alpha.beta");
    allows("1.0.0-rc.1", "1.0.0");
}

#[test]
fn bounded_large_numbers_compare_without_overflow_or_zero_fallback() {
    let huge_core = format!("{}.0.0", "9".repeat(100));
    allows("999.0.0", &huge_core);
    rejects(&huge_core, "999.0.0");

    let huge_pre = format!("1.0.0-{}", "9".repeat(100));
    allows("1.0.0-999", &huge_pre);
    rejects(&huge_pre, "1.0.0-999");
}

#[test]
fn build_metadata_does_not_change_precedence_but_exact_reuse_stays_bound() {
    rejects("1.2.3+build.2", "1.2.3+build.1");
    rejects("1.2.3+build.1", "1.2.3+build.2");
    assert_eq!(
        reject_stale_version_reuse(
            &package("1.2.3+build.1", A, B),
            &package("1.2.3+build.1", B, B),
        )
        .unwrap_err()
        .id(),
        DistributionErrorId::InstallConflict
    );
    reject_stale_version_reuse(
        &package("1.2.3+build.1", A, B),
        &package("1.2.3+build.1", A, B),
    )
    .unwrap();
    reject_stale_version_reuse(
        &package("1.2.3+build.1", A, B),
        &package("1.2.3+build.2", A, B),
    )
    .unwrap();
}

#[test]
fn every_distribution_version_boundary_uses_the_shared_grammar() {
    for invalid in ["1.2.3-01", "1.2.3+", "01.2.3"] {
        assert_eq!(
            CacheExpectation::new(
                CONTEXT_ID.into(),
                CANDIDATE_ID.into(),
                C.into(),
                "local-harness-plugins".into(),
                "harness-ultragoal".into(),
                invalid.into(),
                B.into(),
            )
            .unwrap_err()
            .id(),
            DistributionErrorId::InvalidSpec
        );
        assert_eq!(
            MarketplaceExpectation::new(
                CONTEXT_ID.into(),
                CANDIDATE_ID.into(),
                MarketplaceScope::Repository,
                "harness-ultragoal".into(),
                invalid.into(),
                "plugins/harness-ultragoal.hugpkg".into(),
                B.into(),
            )
            .unwrap_err()
            .id(),
            DistributionErrorId::InvalidSpec
        );
    }

    let fixture = Fixture::complete("invalid-version-envelope");
    fixture.mutate_envelope(Layer::Runtime, |row| {
        row["version"] = serde_json::json!("1.2.3-01");
    });
    assert_eq!(
        verify(&fixture.root, &fixture.bytes()).unwrap_err().id(),
        DistributionErrorId::InvalidSpec
    );
}

#[test]
fn deserialized_invalid_identity_cannot_enter_package_or_host_plans() {
    let source: SourceIdentity = serde_json::from_value(serde_json::json!({
        "context_id": CONTEXT_ID,
        "candidate_id": CANDIDATE_ID,
        "plugin_id": "harness-ultragoal",
        "version": "1.2.3-01",
        "catalog_id": A,
        "accepted_inventory_sha256": A
    }))
    .unwrap();
    assert_eq!(
        PackageIdentity::new(source, A.into(), B.into())
            .unwrap_err()
            .id(),
        DistributionErrorId::InvalidSpec
    );

    let package: PackageIdentity = serde_json::from_value(serde_json::json!({
        "source": {
            "context_id": CONTEXT_ID,
            "candidate_id": CANDIDATE_ID,
            "plugin_id": "harness-ultragoal",
            "version": "1.2.3+",
            "catalog_id": A,
            "accepted_inventory_sha256": A
        },
        "tree_sha256": A,
        "archive_sha256": B
    }))
    .unwrap();
    assert_eq!(
        HostCommandPlan::personal_install(&package, "local-harness-plugins")
            .unwrap_err()
            .id(),
        DistributionErrorId::InvalidSpec
    );
}
