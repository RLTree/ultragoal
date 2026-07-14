static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn require_type<T>() {}

#[test]
fn required_migration_api_types_compile_in_the_leased_module() {
    require_type::<MigrationPlan>();
    require_type::<MigrationPlanProjection>();
    require_type::<CompatibilityRoute>();
    require_type::<RetirementTarget>();
    require_type::<RetirementTargetProjection>();
    require_type::<RetirementDecision>();
}

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn surface(
    id: &str,
    status: SurfaceStatus,
    readers: Vec<String>,
    writers: Vec<String>,
    routes: Vec<String>,
    generated: Vec<String>,
) -> InventorySurface {
    InventorySurface::observed(InventorySurfaceObservation {
        stable_id: id.to_owned(),
        kind: "skill".to_owned(),
        relative_path: format!("skills/{}.md", id.replace(':', "-")),
        digest_sha256: if id.starts_with("LEGACY") {
            sha('a')
        } else {
            sha('b')
        },
        file_kind: SurfaceFileKind::Regular,
        link_count: 1,
        status,
        active_readers: readers,
        active_writers: writers,
        public_routes: routes,
        generated_outputs: generated,
    })
}

fn inventory_with_source(source: InventorySurface, session: char) -> MigrationInventory {
    MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha(session),
        vec![
            source,
            surface(
                "SKILL:current",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ],
    )
    .unwrap()
}

fn clean_inventory() -> MigrationInventory {
    inventory_with_source(
        surface(
            "LEGACY-SKILL:old",
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        'f',
    )
}

fn route(source: &str, target: &str) -> CompatibilityRoute {
    CompatibilityRoute::new(CompatibilityRouteDefinition {
        route_id: "route-old-to-current".to_owned(),
        source_id: source.to_owned(),
        canonical_target_id: target.to_owned(),
        owner_id: "migration-owner".to_owned(),
        warning: "Deprecated compatibility route to the canonical successor".to_owned(),
        usage_measurement_sha256: sha('1'),
        compatibility_boundary: "version-2-boundary".to_owned(),
        removal_condition: "zero-active-references-and-representative-journey".to_owned(),
        equivalence_sha256: sha('2'),
        observed_invocations: 0,
        compatibility_window_complete: true,
    })
}

fn test_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[derive(Default)]
struct TestReplacementLedger {
    consumed: BTreeMap<String, String>,
    plan_bindings: BTreeMap<String, String>,
    final_claims: BTreeMap<String, String>,
    revoked: BTreeSet<String>,
}

#[derive(Clone)]
struct TestReplacementAuthority {
    authority_id: String,
    reviewer_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at: u64,
    expires_at: u64,
    now: u64,
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    route_id: String,
    old_behavior_id: String,
    old_verdict: EvidenceVerdict,
    old_result_sha256: String,
    new_behavior_id: String,
    new_verdict: EvidenceVerdict,
    new_result_sha256: String,
    journey_execution_id: String,
    journey_verdict: EvidenceVerdict,
    journey_result_sha256: String,
    controls: BTreeMap<String, (EvidenceVerdict, String)>,
    rollback_execution_id: String,
    rollback_verdict: EvidenceVerdict,
    rollback_result_sha256: String,
    secret: String,
    ledger: Arc<Mutex<TestReplacementLedger>>,
    mutate_after_consume: bool,
    rollback_after_final_claim: bool,
    revoke_after_final_claim: bool,
    revoke_on_final_verify_call: Option<usize>,
    final_verify_calls: usize,
}
