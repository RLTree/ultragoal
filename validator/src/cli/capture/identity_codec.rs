use crate::context::{CandidateIdentity, LiveContext};
use sha2::{Digest, Sha256};

pub(super) enum IdentityCodecRequest<'a> {
    DigestBytes(&'a [u8]),
    Candidate(&'a CandidateIdentity),
    EncodeHex(&'a [u8]),
    ValidateSha256(&'a str),
}

pub(super) enum IdentityCodecResponse {
    Digest(String),
    EncodedHex(String),
    Validity(bool),
}

#[derive(Debug)]
pub(super) struct IdentityCodecError {
    detail: &'static str,
}

impl IdentityCodecError {
    fn stable_text(self) -> String {
        self.detail.to_owned()
    }
}

pub(super) fn execute(
    request: IdentityCodecRequest<'_>,
) -> Result<IdentityCodecResponse, IdentityCodecError> {
    match request {
        IdentityCodecRequest::DigestBytes(bytes) => {
            Ok(IdentityCodecResponse::Digest(sha256(bytes)))
        }
        IdentityCodecRequest::Candidate(candidate) => serde_json::to_vec(candidate)
            .map(|bytes| IdentityCodecResponse::Digest(sha256(&bytes)))
            .map_err(|_| IdentityCodecError {
                detail: "candidate identity serialization failed",
            }),
        IdentityCodecRequest::EncodeHex(bytes) => {
            let mut encoded = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                use std::fmt::Write;
                let _ = write!(encoded, "{byte:02x}");
            }
            Ok(IdentityCodecResponse::EncodedHex(encoded))
        }
        IdentityCodecRequest::ValidateSha256(value) => {
            let valid = value.len() == 71
                && value.starts_with("sha256:")
                && value[7..]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            Ok(IdentityCodecResponse::Validity(valid))
        }
    }
}

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    match execute(IdentityCodecRequest::DigestBytes(bytes)) {
        Ok(IdentityCodecResponse::Digest(value)) => value,
        _ => unreachable!("digest request has one closed response"),
    }
}

pub(super) fn candidate_id(candidate: &CandidateIdentity) -> Result<String, String> {
    match execute(IdentityCodecRequest::Candidate(candidate))
        .map_err(|error| error.stable_text())?
    {
        IdentityCodecResponse::Digest(value) => Ok(value),
        _ => unreachable!("candidate request has one closed response"),
    }
}

pub(super) fn context_candidate_id(context: &LiveContext) -> Result<String, String> {
    candidate_id(context.candidate())
}

pub(super) fn bytes_hex(bytes: &[u8]) -> String {
    match execute(IdentityCodecRequest::EncodeHex(bytes)) {
        Ok(IdentityCodecResponse::EncodedHex(value)) => value,
        _ => unreachable!("hex request has one closed response"),
    }
}

pub(super) fn valid_sha256(value: &str) -> bool {
    match execute(IdentityCodecRequest::ValidateSha256(value)) {
        Ok(IdentityCodecResponse::Validity(valid)) => valid,
        _ => unreachable!("digest validation request has one closed response"),
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
