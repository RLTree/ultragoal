use crate::distribution::{
    DistributionErrorId, MarketplaceExpectation, MarketplaceScope, MarketplaceVerdict,
    ProvenanceExpectation, SignatureExpectation, SignatureVerifierEffects, unavailable_marketplace,
    verify_marketplace, verify_provenance, verify_signature,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PACKAGE: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

#[test]
fn repository_and_personal_marketplaces_are_verified_separately() {
    for scope in [MarketplaceScope::Repository, MarketplaceScope::Personal] {
        let expected = expectation(scope);
        let bytes = catalog(scope);
        let snapshot = verify_marketplace(&bytes, &expected).expect("catalog");
        assert_eq!(snapshot.verdict(), MarketplaceVerdict::Verified);
        assert_eq!(snapshot.package_sha256(), PACKAGE);
        assert!(snapshot.catalog_sha256().is_some());
    }
    let unavailable = unavailable_marketplace(
        &expectation(MarketplaceScope::Personal),
        "host-marketplace-api-unavailable",
    )
    .expect("explicit unavailable");
    assert_eq!(unavailable.verdict(), MarketplaceVerdict::Unavailable);
    assert!(unavailable.catalog_sha256().is_none());
}

#[test]
fn duplicate_stale_and_wrong_digest_catalogs_fail() {
    let expected = expectation(MarketplaceScope::Repository);
    let mut duplicate: Value =
        serde_json::from_slice(&catalog(MarketplaceScope::Repository)).unwrap();
    duplicate["plugins"].as_array_mut().unwrap().push(json!({
        "plugin_id":"harness-ultragoal","version":"0.0.10",
        "origin":"marketplace/plugin-old.json","package_sha256":PACKAGE
    }));
    assert_eq!(
        verify_marketplace(&serde_json::to_vec(&duplicate).unwrap(), &expected)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict
    );
    let mut wrong: Value = serde_json::from_slice(&catalog(MarketplaceScope::Repository)).unwrap();
    wrong["plugins"][0]["package_sha256"] =
        json!("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert_eq!(
        verify_marketplace(&serde_json::to_vec(&wrong).unwrap(), &expected)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict
    );
}

#[test]
fn exact_origins_block_safe_in_root_substitution_in_both_scopes() {
    for scope in [MarketplaceScope::Repository, MarketplaceScope::Personal] {
        let expected = expectation(scope);
        let mut substituted: Value = serde_json::from_slice(&catalog(scope)).unwrap();
        substituted["plugins"][0]["origin"] = json!("alternate/plugin.json");
        let error =
            verify_marketplace(&serde_json::to_vec(&substituted).unwrap(), &expected).unwrap_err();
        assert_eq!(error.id(), DistributionErrorId::InstallConflict);
        assert!(!error.to_string().contains("alternate"));
    }
}

#[test]
fn traversal_absolute_and_special_origins_fail_without_echo() {
    for origin in [
        "../SECRET_CANARY",
        "/absolute/SECRET_CANARY",
        "marketplace//SECRET_CANARY.json",
        "marketplace\\SECRET_CANARY.json",
        "C:/SECRET_CANARY.json",
        "marketplace/secret\0canary.json",
        "marketplace/secret-§.json",
    ] {
        let expected = expectation(MarketplaceScope::Repository);
        let mut invalid: Value =
            serde_json::from_slice(&catalog(MarketplaceScope::Repository)).unwrap();
        invalid["plugins"][0]["origin"] = json!(origin);
        let error =
            verify_marketplace(&serde_json::to_vec(&invalid).unwrap(), &expected).unwrap_err();
        assert_eq!(error.id(), DistributionErrorId::InvalidPath);
        assert!(!error.to_string().contains("CANARY"));
        assert!(
            MarketplaceExpectation::new(
                CONTEXT_ID.into(),
                CANDIDATE_ID.into(),
                MarketplaceScope::Repository,
                "harness-ultragoal".into(),
                "0.0.11".into(),
                origin.into(),
                PACKAGE.into(),
            )
            .is_err()
        );
    }
    assert!(
        MarketplaceExpectation::new(
            CONTEXT_ID.into(),
            CANDIDATE_ID.into(),
            MarketplaceScope::Repository,
            "harness-ultragoal".into(),
            "0.0.11".into(),
            "a".repeat(513),
            PACKAGE.into(),
        )
        .is_err()
    );
}

#[test]
fn provenance_requires_exact_subject_material_builder_and_candidate() {
    let mut materials = BTreeMap::new();
    materials.insert("git+repo@tree".into(), CONTEXT_ID.into());
    let expected = ProvenanceExpectation {
        context_id: CONTEXT_ID.into(),
        candidate_id: CANDIDATE_ID.into(),
        subject_name: "harness-ultragoal.hugpkg".into(),
        subject_sha256: PACKAGE.into(),
        builder_id: "hct-distribution-v1".into(),
        predicate_type: "https://slsa.dev/provenance/v1".into(),
        materials,
    };
    let valid = provenance();
    verify_provenance(&valid, &expected).expect("provenance");
    let mut tampered: Value = serde_json::from_slice(&valid).unwrap();
    tampered["subject"]["sha256"] = json!(CONTEXT_ID);
    assert_eq!(
        verify_provenance(&serde_json::to_vec(&tampered).unwrap(), &expected)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );
    tampered = serde_json::from_slice(&valid).unwrap();
    tampered["materials"][0]["sha256"] = json!(PACKAGE);
    assert_eq!(
        verify_provenance(&serde_json::to_vec(&tampered).unwrap(), &expected)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );
}

struct Verifier {
    accept: bool,
    calls: usize,
}

impl SignatureVerifierEffects for Verifier {
    fn verify_signature(
        &mut self,
        _envelope: &[u8],
        _subject_sha256: &str,
    ) -> Result<bool, crate::distribution::EffectFailure> {
        self.calls += 1;
        Ok(self.accept)
    }
}

#[test]
fn signatures_require_adopted_expectations_and_real_verifier_effect() {
    let bytes = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.signature-envelope.v1",
        "identity":"release@example.test","issuer":"fixture-issuer","key_id":"fixture-key",
        "subject_sha256":PACKAGE,"signature":"fixture-signature"
    }))
    .unwrap();
    let expected = SignatureExpectation {
        identity: "release@example.test".into(),
        issuer: "fixture-issuer".into(),
        key_id: "fixture-key".into(),
        subject_sha256: PACKAGE.into(),
    };
    let mut verifier = Verifier {
        accept: true,
        calls: 0,
    };
    assert_eq!(
        verify_signature(&bytes, None, &mut verifier)
            .unwrap_err()
            .id(),
        DistributionErrorId::SignaturePolicyUnavailable
    );
    assert_eq!(verifier.calls, 0);
    verify_signature(&bytes, Some(&expected), &mut verifier).expect("signature");
    assert_eq!(verifier.calls, 1);
    let mut rejecting = Verifier {
        accept: false,
        calls: 0,
    };
    assert_eq!(
        verify_signature(&bytes, Some(&expected), &mut rejecting)
            .unwrap_err()
            .id(),
        DistributionErrorId::SignatureMismatch
    );
}

fn expectation(scope: MarketplaceScope) -> MarketplaceExpectation {
    MarketplaceExpectation::new(
        CONTEXT_ID.into(),
        CANDIDATE_ID.into(),
        scope,
        "harness-ultragoal".into(),
        "0.0.11".into(),
        expected_origin(scope).into(),
        PACKAGE.into(),
    )
    .unwrap()
}

fn catalog(scope: MarketplaceScope) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.marketplace-catalog.v1",
        "context_id":CONTEXT_ID,"candidate_id":CANDIDATE_ID,
        "scope":match scope { MarketplaceScope::Repository => "repository", MarketplaceScope::Personal => "personal" },
        "plugins":[{"plugin_id":"harness-ultragoal","version":"0.0.11","origin":expected_origin(scope),"package_sha256":PACKAGE}]
    })).unwrap()
}

fn expected_origin(scope: MarketplaceScope) -> &'static str {
    match scope {
        MarketplaceScope::Repository => "marketplace/repository/plugin.json",
        MarketplaceScope::Personal => "marketplace/personal/plugin.json",
    }
}

fn provenance() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.provenance.v1","context_id":CONTEXT_ID,
        "candidate_id":CANDIDATE_ID,
        "subject":{"name":"harness-ultragoal.hugpkg","sha256":PACKAGE},
        "builder_id":"hct-distribution-v1","predicate_type":"https://slsa.dev/provenance/v1",
        "materials":[{"uri":"git+repo@tree","sha256":CONTEXT_ID}]
    }))
    .unwrap()
}
