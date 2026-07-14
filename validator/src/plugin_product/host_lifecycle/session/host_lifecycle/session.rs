pub struct HostLifecycleSession {
    package: PackageSnapshot,
    lifecycle: LifecyclePlan,
    operation: DistributionLifecycleOperation,
    binding: JourneyBinding,
    marketplace_plan: MarketplacePlan,
    host_scope: Arc<BoundHostScope>,
    capability_gate: Arc<HostEffectCapabilityGate>,
    issuance: Arc<SessionIssuance>,
    external_effect_request: Option<ExternalHostEffectRequest>,
    external_effect_request_sha256: Option<String>,
}

pub struct HostLifecycleBindRequest<'a> {
    pub root: ConfinedRoot,
    pub package_plan: &'a PackagePlan,
    pub package: &'a PackageSnapshot,
    pub lifecycle: LifecyclePlan,
    pub host: HostCapabilityDeclaration,
    pub marketplace_plan: MarketplacePlan,
    pub host_scope: HostScopeAuthority,
}
