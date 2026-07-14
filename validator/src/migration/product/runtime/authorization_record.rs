#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorizationRecord {
    schema_version: String,
    authorization_id: String,
    principal_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    input_binding: MigrationInputBinding,
    plan_sha256: String,
    effect_set_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_binding_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_observation: Option<CompatibilityBoundaryObservation>,
    binding_sha256: String,
    seal_sha256: String,
}

impl fmt::Debug for AuthorizationRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationRecord")
            .field("authorization_id", &self.authorization_id)
            .field("contents", &"<sealed>")
            .finish()
    }
}

impl AuthorizationRecord {
    pub(crate) fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(crate) fn seal_sha256(&self) -> &str {
        &self.seal_sha256
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub(crate) fn input_binding(&self) -> &MigrationInputBinding {
        &self.input_binding
    }

    pub(crate) fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    pub(crate) fn validate_shape(&self) -> bool {
        self.schema_version == "MigrationApplyAuthorizationRecord-v2"
            && valid_sha256(&self.authorization_id)
            && valid_identifier(&self.principal_id)
            && valid_identifier(&self.authority_id)
            && self.principal_id != self.authority_id
            && valid_sha256(&self.authority_session_id)
            && self.authority_session_id != self.input_binding.read_session_id()
            && valid_sha256(&self.nonce_sha256)
            && valid_window(
                self.issued_at_unix_ms,
                self.expires_at_unix_ms,
                self.issued_at_unix_ms,
            )
            && self.input_binding.validate()
            && valid_sha256(&self.plan_sha256)
            && valid_sha256(&self.effect_set_sha256)
            && match (
                self.compatibility_boundary_binding_sha256.as_deref(),
                self.compatibility_boundary_observation.as_ref(),
            ) {
                (Some(binding), Some(observation)) => {
                    valid_sha256(binding) && observation.validate()
                }
                (None, None) => true,
                _ => false,
            }
            && valid_sha256(&self.binding_sha256)
            && valid_sha256(&self.seal_sha256)
            && self.authorization_id
                == digest(
                    format!(
                        "migration-apply-authorization-v2|{}|{}",
                        self.binding_sha256, self.seal_sha256
                    )
                    .as_bytes(),
                )
    }
}

#[derive(Eq, PartialEq)]
pub(crate) struct ApplyAuthorization {
    record: AuthorizationRecord,
}

impl fmt::Debug for ApplyAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplyAuthorization")
            .field("authorization_id", &self.record.authorization_id)
            .field("contents", &"<sealed-single-use>")
            .finish()
    }
}

impl ApplyAuthorization {
    pub(crate) fn authorization_id(&self) -> &str {
        &self.record.authorization_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoreFault {
    code: &'static str,
}

impl StoreFault {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for StoreFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for StoreFault {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationRequest {
    schema_version: String,
    operation_id: String,
    authorization: AuthorizationRecord,
    plan_sha256: String,
    semantic_keys: Vec<String>,
    reservation_sha256: String,
}
