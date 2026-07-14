use super::*;

#[test]
pub(crate) fn mode_drift_invalidates_the_mode_bound_adapter_verification_proof() {
    let fixture = Fixture::new("mode-proof-replay");
    fixture.install_all_direct();
    let current = fixture.context();
    let accepted = verify_target(&current).unwrap();
    assert!(accepted.idempotent());
    let accepted_adapter_proof = accepted.verification_sha256.clone().unwrap();
    let accepted_byte_proof = accepted.byte_verification_sha256.clone().unwrap();
    let accepted_inspection = accepted.inspection_sha256.clone();
    assert_ne!(accepted_adapter_proof, accepted_byte_proof);

    let target = fixture.root.join("scripts/check");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o4755)).unwrap();
    assert_eq!(fs::metadata(&target).unwrap().mode() & 0o7777, 0o4755);
    let drifted = verify_target(&fixture.context()).unwrap();

    assert!(!drifted.idempotent());
    assert_eq!(drifted.matched_files(), CANONICAL_TEMPLATES.len() - 1);
    assert!(drifted.verification_sha256.is_none());
    assert!(drifted.byte_verification_sha256.is_none());
    assert_ne!(drifted.inspection_sha256, accepted_inspection);
    assert!(
        !drifted
            .to_machine_bytes()
            .unwrap()
            .windows(accepted_adapter_proof.len())
            .any(|window| window == accepted_adapter_proof.as_bytes())
    );
}

#[test]
pub(crate) fn direct_already_fitted_fixture_verifies_without_writes() {
    let fixture = Fixture::new("already-fit");
    fixture.install_all_direct();
    let context = fixture.context();
    let verification = assert_zero_write(&fixture, || verify_target(&context).unwrap());
    assert!(verification.idempotent());
    assert!(verification.failure.is_none());
}

#[test]
pub(crate) fn mismatch_verify_reports_causal_files_and_preserves_every_recursive_attribute() {
    let fixture = Fixture::new("verify-failure");
    fixture.write_template("AGENTS.md");
    let context = fixture.context();
    let verification = assert_zero_write(&fixture, || verify_target(&context).unwrap());
    assert!(!verification.idempotent());
    assert_eq!(verification.matched_files(), 1);
    let failure = verification.failure.unwrap();
    assert_eq!(failure.error_id, "HUFIT-011");
    assert_eq!(failure.classification, "partial");
    assert_eq!(failure.causal_files.len(), CANONICAL_TEMPLATES.len() - 1);
}

#[test]
pub(crate) fn failed_apply_rolls_back_files_directories_permissions_links_and_git_status() {
    let fixture = Fixture::new("rollback");
    fixture.write("keep/data", b"untouched\n");
    let context = fixture.context();
    let prepared = fixture.plan(&context);
    let status = git_status(&fixture.root);
    let before = snapshot(&fixture.root);
    let mut effects = effects_for(&fixture, &prepared);
    effects.fail_on_calls([2]);
    let failure = prepared
        .into_request()
        .execute_for_test(&mut effects)
        .unwrap_err();
    assert_eq!(failure.id(), FitErrorId::EffectFailed);
    assert_eq!(git_status(&fixture.root), status);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn rollback_ambiguity_is_reported_and_a_fresh_partial_plan_recovers() {
    let fixture = Fixture::new("recovery");
    let context = fixture.context();
    let prepared = fixture.plan(&context);
    let mut effects = effects_for(&fixture, &prepared);
    effects.fail_on_calls([2, 3]);
    let failure = prepared
        .into_request()
        .execute_for_test(&mut effects)
        .unwrap_err();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);

    let partial_context = fixture.context();
    let partial = inspect_target(&partial_context).unwrap();
    assert_eq!(partial.classification(), "partial");
    let recovery = fixture.plan(&partial_context);
    execute(&fixture, recovery).unwrap();
    let final_context = fixture.context();
    assert!(verify_target(&final_context).unwrap().idempotent());
}

#[test]
pub(crate) fn existing_file_atomic_replacement_applies_authoritative_mode_and_rolls_it_back() {
    use crate::repository_fit::{CanonicalPath, ExpectedContent, FitEffects, digest};
    use std::collections::BTreeMap;

    let fixture = Fixture::new("replace-existing");
    fixture.write("single.txt", b"prior");
    fs::set_permissions(
        fixture.root.join("single.txt"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let mut effects = crate::repository_fit::local::LocalEffects::open_for_test(
        &fixture.root,
        BTreeMap::from([("single.txt".to_owned(), 0o644)]),
    )
    .unwrap();
    assert!(
        effects
            .compare_exchange(
                &CanonicalPath::parse("single.txt").unwrap(),
                &ExpectedContent::ExactDigest(digest(b"prior")),
                Some(b"next"),
            )
            .unwrap()
    );
    let metadata = fs::metadata(fixture.root.join("single.txt")).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o644);
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(fs::read(fixture.root.join("single.txt")).unwrap(), b"next");
    assert!(
        effects
            .compare_exchange(
                &CanonicalPath::parse("single.txt").unwrap(),
                &ExpectedContent::ExactDigest(digest(b"next")),
                Some(b"prior"),
            )
            .unwrap()
    );
    let rolled_back = fs::metadata(fixture.root.join("single.txt")).unwrap();
    assert_eq!(rolled_back.mode() & 0o7777, 0o600);
    assert_eq!(rolled_back.nlink(), 1);
    assert_eq!(fs::read(fixture.root.join("single.txt")).unwrap(), b"prior");
}

#[test]
pub(crate) fn public_class_names_cover_the_accepted_kernel_without_inventing_a_state() {
    assert_eq!(RepositoryClass::Fresh, RepositoryClass::Fresh);
    assert_eq!(RepositoryClass::Partial, RepositoryClass::Partial);
    assert_eq!(RepositoryClass::Conflicting, RepositoryClass::Conflicting);
    assert_eq!(
        RepositoryClass::AlreadyFitted,
        RepositoryClass::AlreadyFitted
    );
}
