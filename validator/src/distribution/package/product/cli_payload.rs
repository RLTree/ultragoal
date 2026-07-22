use super::spec::CLI_ENTRY_LIMIT;
use crate::distribution::spec::digest;

/// Bytes for the exact CLI build selected by the caller for one source candidate.
///
/// This type deliberately accepts only native executable payloads. Shell scripts,
/// path references, and probe-like text cannot become the installed CLI entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateCliPayload {
    candidate_id: String,
    sha256: String,
    bytes: Vec<u8>,
}

impl CandidateCliPayload {
    pub fn for_candidate(
        candidate_id: &str,
        bytes: Vec<u8>,
    ) -> Result<Self, ProductionPackageError> {
        if !digest(candidate_id)
            || bytes.is_empty()
            || bytes.len() > CLI_ENTRY_LIMIT
            || !is_native_executable(&bytes)
        {
            return Err(failure(ProductionPackageErrorId::MembershipMismatch));
        }
        Ok(Self {
            candidate_id: candidate_id.to_owned(),
            sha256: sha256(&bytes),
            bytes,
        })
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn is_native_executable(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x7fELF")
        || bytes.starts_with(b"MZ")
        || matches!(
            bytes.get(..4),
            Some(
                [0xfe, 0xed, 0xfa, 0xce]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xbe, 0xba, 0xfe, 0xca]
            )
        )
}
