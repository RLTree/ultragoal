use super::artifact_model::ArtifactDisposition;
#[cfg(test)]
use super::environment::InvocationSensitivity;
use super::identity_codec::digest_bytes;
use std::sync::Arc;

pub(super) struct FinalizedArtifact {
    pub disposition: ArtifactDisposition,
    pub bytes: Arc<[u8]>,
    pub sha256: String,
}

pub(super) fn public(bytes: Arc<[u8]>, sha256: String) -> FinalizedArtifact {
    FinalizedArtifact {
        disposition: ArtifactDisposition::Public,
        bytes,
        sha256,
    }
}

pub(super) fn withheld_secret_bearing_invocation() -> FinalizedArtifact {
    let bytes: Arc<[u8]> = Arc::from([]);
    FinalizedArtifact {
        disposition: ArtifactDisposition::WithheldSecretBearingInvocation,
        sha256: digest_bytes(&bytes),
        bytes,
    }
}

#[cfg(test)]
pub(crate) fn finalize_for_test(
    bytes: &[u8],
    secrets: &[Vec<u8>],
) -> (ArtifactDisposition, Arc<[u8]>, String) {
    let finalized = match InvocationSensitivity::from_bound_secrets(secrets) {
        InvocationSensitivity::Public => {
            let bytes: Arc<[u8]> = Arc::from(bytes);
            public(Arc::clone(&bytes), digest_bytes(&bytes))
        }
        InvocationSensitivity::SecretBearing => withheld_secret_bearing_invocation(),
    };
    (finalized.disposition, finalized.bytes, finalized.sha256)
}
