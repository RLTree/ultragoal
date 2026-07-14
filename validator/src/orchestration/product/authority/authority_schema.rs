const AUTHORITY_SCHEMA: &str = "OrchestrationRootPermit-v2";
const AUTHORITY_DOMAIN: &[u8] = b"harness-ultragoal/orchestration-root-permit/v2\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootOperation {
    Resume,
    Reconcile,
    Recover,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermitTarget {
    pub lease_id: Option<String>,
    pub result_commitment_id: Option<String>,
    pub operation_id: Option<String>,
    pub recovered_binding: Option<Binding>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PermitDecisionBinding {
    ActionOnly,
    ReconcileEffect {
        effect_resolution_commitment_id: String,
    },
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootPermit {
    schema_version: String,
    root_actor: String,
    operation: RootOperation,
    binding: Binding,
    workspace_identity: String,
    journal_head_identity: String,
    issued_tick: u64,
    expires_tick: u64,
    nonce_digest: String,
    target: PermitTarget,
    decision_binding: PermitDecisionBinding,
    authenticator: String,
}

impl Debug for RootPermit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootPermit")
            .field("schema_version", &self.schema_version)
            .field("root_actor", &self.root_actor)
            .field("operation", &self.operation)
            .field("binding", &self.binding)
            .field("workspace_identity", &self.workspace_identity)
            .field("journal_head_identity", &self.journal_head_identity)
            .field("issued_tick", &self.issued_tick)
            .field("expires_tick", &self.expires_tick)
            .field("nonce_digest", &self.nonce_digest)
            .field("target", &self.target)
            .field("decision_binding", &"[bound]")
            .field("authenticator", &"[redacted]")
            .finish()
    }
}

pub(crate) struct RootAuthority {
    root_actor: Actor,
    key: [u8; 32],
}

impl Debug for RootAuthority {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootAuthority")
            .field("root_actor", &self.root_actor.as_str())
            .field("key", &"[redacted]")
            .finish()
    }
}
