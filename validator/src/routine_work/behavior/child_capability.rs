use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::super::digest::{canonical, framed, valid};
use super::super::{RoutineError, RoutineErrorId};

pub(crate) const CHILD_CAPABILITY_ENV: &str = "HUL_ROUTINE_CHILD_FD";
pub(crate) const CHILD_CAPABILITY_FD: i32 = 198;
pub(crate) const MAX_CHILD_CAPABILITY_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoutineChildCapability {
    pub(crate) schema_version: String,
    pub(crate) behavior_id: String,
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) node_id: String,
    pub(crate) grant_session_id: String,
    pub(crate) grant_id: String,
    pub(crate) reservation_marker: String,
    pub(crate) parent_pid: i32,
    pub(crate) child_pid: i32,
    pub(crate) process_session_id: i32,
    pub(crate) program_path_hex: String,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: Option<u32>,
    pub(crate) program_device: u64,
    pub(crate) program_inode: u64,
    pub(crate) program_changed_seconds: i64,
    pub(crate) program_changed_nanos: i64,
    pub(crate) framed_input_sha256: String,
    pub(crate) nonce_hex: String,
    pub(crate) capability_seal: String,
}

#[derive(Serialize)]
struct CapabilitySealPayload<'a> {
    domain: &'static str,
    capability: &'a RoutineChildCapability,
}

impl RoutineChildCapability {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, RoutineError> {
        self.validate_shape()?;
        canonical(self)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, RoutineError> {
        let value: Self = serde_json::from_slice(bytes).map_err(|_| error())?;
        if canonical(&value)?.as_slice() != bytes {
            return Err(error());
        }
        value.validate_shape()?;
        Ok(value)
    }

    pub(crate) fn seal(&mut self, secret: &[u8; 32]) -> Result<(), RoutineError> {
        self.capability_seal = self.expected_seal(secret)?;
        Ok(())
    }

    pub(crate) fn verify_seal(&self, secret: &[u8; 32]) -> Result<(), RoutineError> {
        if self.capability_seal != self.expected_seal(secret)? {
            return Err(error());
        }
        Ok(())
    }

    pub(crate) fn acknowledgement(&self) -> String {
        framed(&[
            b"routine-child-capability-ack-v1",
            self.nonce_hex.as_bytes(),
            self.child_pid.to_string().as_bytes(),
            self.reservation_marker.as_bytes(),
        ])
    }

    fn validate_shape(&self) -> Result<(), RoutineError> {
        let bounded = [
            self.behavior_id.as_str(),
            self.intent_id.as_str(),
            self.node_id.as_str(),
        ]
        .into_iter()
        .all(|value| !value.is_empty() && value.len() <= 256);
        if self.schema_version != "RoutineChildCapability-v1"
            || !bounded
            || ![
                &self.request_id,
                &self.protocol_id,
                &self.grant_session_id,
                &self.grant_id,
                &self.reservation_marker,
                &self.program_sha256,
                &self.framed_input_sha256,
                &self.capability_seal,
            ]
            .into_iter()
            .all(|value| valid(value))
            || self.parent_pid <= 1
            || self.child_pid <= 1
            || self.process_session_id <= 0
            || self.program_path_hex.is_empty()
            || self.program_path_hex.len() > 16 * 1024
            || self.program_path_hex.len() % 2 != 0
            || !self
                .program_path_hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.program_byte_length == 0
            || self.program_unix_mode.is_none()
            || self.program_device == 0
            || self.program_inode == 0
            || self.nonce_hex.len() != 64
            || !self.nonce_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.reservation_marker
                != framed(&[
                    b"routine-mediated-recovery-v1",
                    self.grant_id.as_bytes(),
                    self.protocol_id.as_bytes(),
                    self.request_id.as_bytes(),
                ])
        {
            return Err(error());
        }
        Ok(())
    }

    fn expected_seal(&self, secret: &[u8; 32]) -> Result<String, RoutineError> {
        let mut capability = self.clone();
        capability.capability_seal.clear();
        let payload = canonical(&CapabilitySealPayload {
            domain: "routine-child-capability-seal-v1",
            capability: &capability,
        })?;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| error())?;
        mac.update(&payload);
        Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
    }
}

fn error() -> RoutineError {
    RoutineError::new(
        RoutineErrorId::InvalidRequest,
        "routine-child-capability-invalid",
        None,
    )
}
