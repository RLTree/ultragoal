pub(crate) trait HostTargetObserver {
    fn acquire(
        &mut self,
        expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError>;
}

struct BoundAuthority {
    authority: HostEffectAuthority,
    issuer_id: String,
    ledger_id: String,
    binding_sha256: String,
}

impl BoundAuthority {
    fn generate(issuer_id: String, ledger_id: String) -> Result<Self, SupportedHostLifecycleError> {
        let authority = HostEffectAuthority::generate(issuer_id.clone(), ledger_id.clone())
            .map_err(|_| authority_rejected())?;
        let mut nonce = [0_u8; AUTHORITY_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| authority_rejected())?;
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            issuer_id: &'a str,
            ledger_id: &'a str,
            instance_nonce_sha256: String,
        }
        let binding_sha256 = digest_json(&Binding {
            schema: "harness-ultragoal.root-host-effect-authority-instance.v1",
            issuer_id: &issuer_id,
            ledger_id: &ledger_id,
            instance_nonce_sha256: digest_bytes(&nonce),
        })?;
        nonce.fill(0);
        Ok(Self {
            authority,
            issuer_id,
            ledger_id,
            binding_sha256,
        })
    }
}

struct BoundLedger<'a> {
    ledger: &'a dyn DurableHostEffectLedger,
    ledger_id: String,
    binding_sha256: String,
}

impl<'a> BoundLedger<'a> {
    fn bind(
        ledger: &'a dyn DurableHostEffectLedger,
        ledger_id: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_id(&ledger_id) {
            return Err(invalid());
        }
        let mut nonce = [0_u8; LEDGER_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| invalid())?;
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            ledger_id: &'a str,
            instance_nonce_sha256: String,
        }
        let binding_sha256 = digest_json(&Binding {
            schema: "harness-ultragoal.bound-host-effect-ledger-instance.v1",
            ledger_id: &ledger_id,
            instance_nonce_sha256: digest_bytes(&nonce),
        })?;
        nonce.fill(0);
        Ok(Self {
            ledger,
            ledger_id,
            binding_sha256,
        })
    }
}

pub(crate) struct SupportedHostLifecycleCoordinator<'a> {
    authority: BoundAuthority,
    ledger: BoundLedger<'a>,
}
