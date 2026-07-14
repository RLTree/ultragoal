use super::HostEffectReservation;
use getrandom::fill;
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};

include!("hmac_sha256.rs");

include!("host_effect_permit_issuer_id.rs");

include!("is_digest.rs");

#[cfg(test)]
mod tests;
