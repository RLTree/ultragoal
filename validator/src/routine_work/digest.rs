use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{RoutineError, RoutineErrorId};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn framed(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn canonical<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, RoutineError> {
    serde_json::to_vec(value).map_err(|_| {
        RoutineError::new(
            RoutineErrorId::Serialization,
            "canonical-serialization-failed",
            None,
        )
    })
}

pub(crate) fn digest_of<T: Serialize + ?Sized>(value: &T) -> Result<String, RoutineError> {
    canonical(value).map(|bytes| sha256(&bytes))
}

pub(crate) fn valid(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
