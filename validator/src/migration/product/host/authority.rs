use super::{HostContext, HostError, TrustedTime, digest_bytes};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::{Arc, Mutex};

use super::super::{
    ApplyAuthorizationAuthority, MigrationInputBinding, MigrationOperation, ProductMigrationError,
    ProductMigrationPlan,
};
use serde::Deserialize;

const AUTHORIZATION_SEAL_DOMAIN: &str = "harness-ultragoal.migration-authorization-seal.v1";
const BOUNDARY_SEAL_DOMAIN: &str = "harness-ultragoal.migration-boundary-seal.v1";
const SESSION_DOMAIN: &str = "harness-ultragoal.migration-authority-session.v1";
const NONCE_DOMAIN: &str = "harness-ultragoal.migration-authority-nonce.v1";
const AUTHORIZATION_TTL_MS: u64 = 5 * 60 * 1_000;

pub(super) struct DarwinMigrationAuthorityFactory {
    context: Arc<HostContext>,
}

impl DarwinMigrationAuthorityFactory {
    pub(super) fn new(context: Arc<HostContext>) -> Self {
        Self { context }
    }

    pub(super) fn boundary_authority(&self) -> Result<DarwinMigrationAuthority, HostError> {
        let placeholder = digest_bytes(b"migration-host-boundary-only-binding");
        DarwinMigrationAuthority::issue(
            self.context.clone(),
            placeholder.clone(),
            placeholder,
            None,
        )
    }

    pub(super) fn apply_authority(
        &self,
        plan: &ProductMigrationPlan,
    ) -> Result<DarwinMigrationAuthority, HostError> {
        plan.validate()
            .map_err(|_| HostError::new("migration-host-plan-invalid"))?;
        if plan.input_binding().candidate_id() != self.context.config().candidate_id {
            return Err(HostError::new("migration-host-plan-candidate-substituted"));
        }
        DarwinMigrationAuthority::issue(
            self.context.clone(),
            plan.input_binding().binding_sha256().to_owned(),
            plan.plan_sha256().to_owned(),
            Some(plan.input_binding().read_session_id()),
        )
    }

    pub(super) fn recovery_authority(
        &self,
        operation: &MigrationOperation,
        plan: &ProductMigrationPlan,
    ) -> Result<DarwinMigrationAuthority, HostError> {
        operation
            .validate_shape()
            .then_some(())
            .ok_or_else(|| HostError::new("migration-host-recovery-operation-invalid"))?;
        plan.validate()
            .map_err(|_| HostError::new("migration-host-plan-invalid"))?;
        let encoded = serde_json::to_vec(operation)
            .map_err(|_| HostError::new("migration-host-recovery-operation-invalid"))?;
        let durable: DurableRecoveryEnvelope = serde_json::from_slice(&encoded)
            .map_err(|_| HostError::new("migration-host-recovery-operation-invalid"))?;
        if durable.plan_sha256 != plan.plan_sha256()
            || durable.input_binding != *plan.input_binding()
            || durable.authorization.principal_id != self.context.config().principal_id
            || durable.authorization.authority_id
                != self.context.config().authorization_authority_id
            || !super::super::super::valid_sha256(&durable.authorization.authority_session_id)
            || durable.authorization.authority_session_id == plan.input_binding().read_session_id()
            || !super::super::super::valid_sha256(&durable.authorization.nonce_sha256)
            || !super::super::super::valid_sha256(&durable.authorization.binding_sha256)
            || !super::super::super::valid_sha256(&durable.authorization.seal_sha256)
        {
            return Err(HostError::new(
                "migration-host-recovery-authority-substituted",
            ));
        }
        let recovery_binding_sha256 = durable.authorization.binding_sha256.clone();
        let recovery_seal_sha256 = durable.authorization.seal_sha256.clone();
        let authority = DarwinMigrationAuthority::recover(
            self.context.clone(),
            plan.input_binding().binding_sha256().to_owned(),
            plan.plan_sha256().to_owned(),
            durable.authorization,
        )?;
        if !authority.verify_value(
            AUTHORIZATION_SEAL_DOMAIN,
            &recovery_binding_sha256,
            &recovery_seal_sha256,
        ) {
            return Err(HostError::new(
                "migration-host-recovery-authority-seal-refused",
            ));
        }
        Ok(authority)
    }
}

#[derive(Deserialize)]
struct DurableRecoveryEnvelope {
    plan_sha256: String,
    input_binding: MigrationInputBinding,
    authorization: DurableRecoveryAuthorization,
}

#[derive(Deserialize)]
struct DurableRecoveryAuthorization {
    principal_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    binding_sha256: String,
    seal_sha256: String,
}

struct BoundaryState {
    time: TrustedTime,
    refresh_on_next_sequence: bool,
}

pub(crate) struct DarwinMigrationAuthority {
    context: Arc<HostContext>,
    session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    input_binding_sha256: String,
    plan_sha256: String,
    boundary: Mutex<BoundaryState>,
}

impl DarwinMigrationAuthority {
    fn issue(
        context: Arc<HostContext>,
        input_binding_sha256: String,
        plan_sha256: String,
        forbidden_session: Option<&str>,
    ) -> Result<Self, HostError> {
        context.verify_static()?;
        if !super::super::super::valid_sha256(&input_binding_sha256)
            || !super::super::super::valid_sha256(&plan_sha256)
        {
            return Err(HostError::new("migration-host-authority-binding-invalid"));
        }
        let time = context.trusted_time()?;
        let session_id = random_digest(SESSION_DOMAIN)?;
        if forbidden_session == Some(session_id.as_str()) {
            return Err(HostError::new("migration-host-authority-session-collision"));
        }
        let nonce_sha256 = random_digest(NONCE_DOMAIN)?;
        let expires_at_unix_ms = time
            .unix_ms
            .checked_add(AUTHORIZATION_TTL_MS)
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        Ok(Self {
            context,
            session_id,
            nonce_sha256,
            issued_at_unix_ms: time.unix_ms,
            expires_at_unix_ms,
            input_binding_sha256,
            plan_sha256,
            boundary: Mutex::new(BoundaryState {
                time,
                refresh_on_next_sequence: false,
            }),
        })
    }

    fn recover(
        context: Arc<HostContext>,
        input_binding_sha256: String,
        plan_sha256: String,
        durable: DurableRecoveryAuthorization,
    ) -> Result<Self, HostError> {
        context.verify_static()?;
        let time = context.trusted_time()?;
        if durable.issued_at_unix_ms == 0
            || durable.expires_at_unix_ms <= durable.issued_at_unix_ms
            || durable.expires_at_unix_ms - durable.issued_at_unix_ms > 10 * 60 * 1_000
        {
            return Err(HostError::new(
                "migration-host-recovery-authority-window-refused",
            ));
        }
        Ok(Self {
            context,
            session_id: durable.authority_session_id,
            nonce_sha256: durable.nonce_sha256,
            issued_at_unix_ms: durable.issued_at_unix_ms,
            expires_at_unix_ms: durable.expires_at_unix_ms,
            input_binding_sha256,
            plan_sha256,
            boundary: Mutex::new(BoundaryState {
                time,
                refresh_on_next_sequence: false,
            }),
        })
    }

    fn current_boundary_time(&self, refresh: bool) -> TrustedTime {
        let mut boundary = self
            .boundary
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if refresh && boundary.refresh_on_next_sequence {
            boundary.time = self.context.trusted_time().unwrap_or(TrustedTime {
                monotonic_ns: u64::MAX,
                unix_ms: u64::MAX,
            });
            boundary.refresh_on_next_sequence = false;
        }
        boundary.time
    }

    fn mark_boundary_observed(&self, binding_sha256: &str) {
        let mut boundary = self
            .boundary
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if boundary_binding(
            self.context.config(),
            boundary.time.monotonic_ns,
            boundary.time.unix_ms,
        ) == binding_sha256
        {
            boundary.refresh_on_next_sequence = true;
        }
    }

    fn seal_value(&self, domain: &str, binding_sha256: &str) -> Result<String, HostError> {
        self.context.verify_static()?;
        if !super::super::super::valid_sha256(binding_sha256) {
            return Err(HostError::new("migration-host-seal-binding-invalid"));
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(self.context.key())
            .map_err(|_| HostError::new("migration-host-secret-key-invalid"))?;
        mac.update(domain.as_bytes());
        mac.update(b"|");
        mac.update(binding_sha256.as_bytes());
        Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
    }

    fn verify_value(&self, domain: &str, binding_sha256: &str, seal_sha256: &str) -> bool {
        self.seal_value(domain, binding_sha256)
            .map(|expected| expected == seal_sha256)
            .unwrap_or(false)
    }
}

impl ApplyAuthorizationAuthority for DarwinMigrationAuthority {
    fn principal_id(&self) -> &str {
        &self.context.config().principal_id
    }

    fn authority_id(&self) -> &str {
        &self.context.config().authorization_authority_id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }

    fn issued_at_unix_ms(&self) -> u64 {
        self.issued_at_unix_ms
    }

    fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    fn now_unix_ms(&self) -> u64 {
        self.context
            .trusted_time()
            .map(|time| time.unix_ms)
            .unwrap_or(u64::MAX)
    }

    fn current_binding(&self) -> (&str, &str) {
        (&self.input_binding_sha256, &self.plan_sha256)
    }

    fn seal(&mut self, binding_sha256: &str) -> Result<String, ProductMigrationError> {
        self.seal_value(AUTHORIZATION_SEAL_DOMAIN, binding_sha256)
            .map_err(HostError::product)
    }

    fn verify_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        self.verify_value(AUTHORIZATION_SEAL_DOMAIN, binding_sha256, seal_sha256)
    }

    fn compatibility_boundary_authority_id(&self) -> &str {
        &self.context.config().compatibility_boundary_authority_id
    }

    fn compatibility_boundary_source_identity_sha256(&self) -> &str {
        &self
            .context
            .config()
            .compatibility_boundary_source_identity_sha256
    }

    fn compatibility_boundary_observation_sequence(&self) -> u64 {
        self.current_boundary_time(true).monotonic_ns
    }

    fn compatibility_boundary_observed_at_unix_ms(&self) -> u64 {
        self.current_boundary_time(false).unix_ms
    }

    fn compatibility_boundary_current_product_version(&self) -> &str {
        &self.context.config().product_version
    }

    fn seal_compatibility_boundary(
        &self,
        binding_sha256: &str,
    ) -> Result<String, ProductMigrationError> {
        self.seal_value(BOUNDARY_SEAL_DOMAIN, binding_sha256)
            .map_err(HostError::product)
    }

    fn verify_compatibility_boundary_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        let valid = self.verify_value(BOUNDARY_SEAL_DOMAIN, binding_sha256, seal_sha256);
        if valid {
            self.mark_boundary_observed(binding_sha256);
        }
        valid
    }
}

fn random_digest(domain: &str) -> Result<String, HostError> {
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random)
        .map_err(|_| HostError::new("migration-host-random-unavailable"))?;
    let mut bytes = domain.as_bytes().to_vec();
    bytes.push(b'|');
    bytes.extend_from_slice(&random);
    Ok(digest_bytes(&bytes))
}

fn boundary_binding(
    config: &super::HostAuthorityConfig,
    sequence: u64,
    observed_at_unix_ms: u64,
) -> String {
    digest_bytes(
        format!(
            "migration-compatibility-boundary-observation-binding-v1|{}|{}|{}|{}|{}",
            config.compatibility_boundary_authority_id,
            config.compatibility_boundary_source_identity_sha256,
            sequence,
            observed_at_unix_ms,
            config.product_version,
        )
        .as_bytes(),
    )
}
