use super::artifact_secret_support::digest;
use super::capture::{ArtifactDisposition, finalize_for_test};

#[test]
fn any_nonempty_bound_secret_withholds_without_content_comparison() {
    let bytes = b"ax";
    let secrets = [b"abzz".to_vec()];

    let (disposition, retained, observed_digest) = finalize_for_test(bytes, &secrets);
    assert_eq!(
        disposition,
        ArtifactDisposition::WithheldSecretBearingInvocation
    );
    assert!(retained.is_empty());
    assert_eq!(observed_digest, digest(&[]));
    assert_ne!(observed_digest, digest(bytes));
}

#[test]
fn defensive_empty_secret_does_not_create_a_secret_bearing_invocation() {
    let bytes = b"ordinary-public-artifact";
    let (disposition, retained, observed_digest) = finalize_for_test(bytes, &[Vec::new()]);
    assert_eq!(disposition, ArtifactDisposition::Public);
    assert_eq!(&*retained, bytes);
    assert_eq!(observed_digest, digest(bytes));
}

#[test]
fn explicit_secret_never_uses_the_raw_secret_digest_as_a_substitute() {
    let secret = b"zzq7V5-no-raw-digest-substitution".to_vec();
    let raw_secret_digest = digest(&secret);
    let (disposition, retained, observed_digest) =
        finalize_for_test(&secret, std::slice::from_ref(&secret));
    assert_eq!(
        disposition,
        ArtifactDisposition::WithheldSecretBearingInvocation
    );
    assert!(retained.is_empty());
    assert_eq!(observed_digest, digest(&[]));
    assert_ne!(observed_digest, raw_secret_digest);
}
