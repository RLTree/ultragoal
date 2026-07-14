#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostScopeAuthority {
    Personal {
        marketplace: String,
    },
    Repository {
        repository_root: String,
        marketplace: String,
    },
}

impl HostScopeAuthority {
    pub const fn marketplace_scope(&self) -> MarketplaceScope {
        match self {
            Self::Personal { .. } => MarketplaceScope::Personal,
            Self::Repository { .. } => MarketplaceScope::Repository,
        }
    }

    pub fn marketplace(&self) -> &str {
        match self {
            Self::Personal { marketplace } | Self::Repository { marketplace, .. } => marketplace,
        }
    }

    pub const fn accepts_marketplace_scope(&self, scope: MarketplaceScope) -> bool {
        matches!(
            (self, scope),
            (Self::Personal { .. }, MarketplaceScope::Personal)
                | (Self::Repository { .. }, MarketplaceScope::Repository)
        )
    }
}

#[derive(Serialize)]
pub struct ExternalHostEffectRequest {
    intent: LifecycleIntent,
    binding_sha256: String,
    session_issuance_sha256: String,
    host_scope_sha256: String,
    capability_gate_sha256: String,
    lifecycle_plan_id: String,
    plan_sha256: String,
    request_sha256: String,
    #[serde(skip)]
    plan: HostCommandPlan,
    #[serde(skip)]
    issuance: Arc<SessionIssuance>,
    #[serde(skip)]
    capability_gate: Arc<HostEffectCapabilityGate>,
    #[serde(skip)]
    host_scope: Arc<BoundHostScope>,
}

pub struct PreparedExternalHostEffect {
    intent: LifecycleIntent,
    binding_sha256: String,
    session_issuance_sha256: String,
    host_scope_sha256: String,
    capability_gate_sha256: String,
    request_sha256: String,
    plan: HostCommandPlan,
    issuance: Arc<SessionIssuance>,
    capability_gate: Arc<HostEffectCapabilityGate>,
    host_scope: Arc<BoundHostScope>,
}
