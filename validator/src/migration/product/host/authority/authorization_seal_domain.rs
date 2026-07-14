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
