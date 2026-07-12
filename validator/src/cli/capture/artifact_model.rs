use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactDisposition {
    Public,
    WithheldSecretBearingInvocation,
}

/// Immutable bytes held by the capture session's content-addressed store.
#[derive(Clone, Serialize)]
pub struct CapturedArtifact {
    pub(super) schema_version: &'static str,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) relative_path_hex: Option<String>,
    pub(super) content_disposition: ArtifactDisposition,
    pub(super) content_sha256: String,
    pub(super) byte_length: u64,
    pub(super) store_identity: &'static str,
    pub(super) resolver_identity: &'static str,
    #[serde(skip)]
    pub(super) relative_path: Option<PathBuf>,
    #[serde(skip)]
    pub(super) bytes: Arc<[u8]>,
}

impl fmt::Debug for CapturedArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapturedArtifact")
            .field("schema_version", &self.schema_version)
            .field("context_id", &self.context_id)
            .field("candidate_id", &self.candidate_id)
            .field("relative_path_hex", &self.relative_path_hex)
            .field("content_disposition", &self.content_disposition)
            .field("content_sha256", &self.content_sha256)
            .field("byte_length", &self.byte_length)
            .field("store_identity", &self.store_identity)
            .field("resolver_identity", &self.resolver_identity)
            .finish()
    }
}

impl CapturedArtifact {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn relative_path(&self) -> Option<&Path> {
        self.relative_path.as_deref()
    }

    pub fn sha256(&self) -> &str {
        &self.content_sha256
    }

    pub fn disposition(&self) -> ArtifactDisposition {
        self.content_disposition
    }

    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "captured artifact serialization failed".to_owned())
    }

    pub fn artifact_ref(&self) -> ArtifactRef {
        ArtifactRef {
            schema_version: "ArtifactRef-v1",
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            content_disposition: self.content_disposition,
            content_sha256: self.content_sha256.clone(),
            byte_length: self.byte_length,
            store_identity: self.store_identity,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArtifactRef {
    schema_version: &'static str,
    context_id: String,
    candidate_id: String,
    content_disposition: ArtifactDisposition,
    content_sha256: String,
    byte_length: u64,
    store_identity: &'static str,
}

impl ArtifactRef {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn sha256(&self) -> &str {
        &self.content_sha256
    }

    pub fn disposition(&self) -> ArtifactDisposition {
        self.content_disposition
    }

    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "artifact reference serialization failed".to_owned())
    }

    fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() as u64 != self.byte_length
            || super::util::digest_bytes(bytes) != self.content_sha256
        {
            return Err("resolved artifact bytes do not match ArtifactRef".to_owned());
        }
        Ok(())
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Sealed resolver: callers can resolve only through a capture-owned store.
pub trait ArtifactResolver: sealed::Sealed {
    fn resolve(&self, reference: &ArtifactRef) -> Result<Arc<[u8]>, String>;
}

impl sealed::Sealed for CapturedArtifact {}

impl ArtifactResolver for CapturedArtifact {
    fn resolve(&self, reference: &ArtifactRef) -> Result<Arc<[u8]>, String> {
        if reference != &self.artifact_ref() {
            return Err("ArtifactRef does not belong to this capture store".to_owned());
        }
        reference.verify(&self.bytes)?;
        Ok(Arc::clone(&self.bytes))
    }
}
