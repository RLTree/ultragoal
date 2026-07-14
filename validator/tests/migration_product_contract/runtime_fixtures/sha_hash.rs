pub(crate) fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn surface(
    id: &str,
    kind: &str,
    path: &str,
    digest_byte: char,
    status: SurfaceStatus,
    readers: &[&str],
    writers: &[&str],
    routes: &[&str],
    generated: &[&str],
) -> InventorySurface {
    InventorySurface::observed(InventorySurfaceObservation {
        stable_id: id.to_owned(),
        kind: kind.to_owned(),
        relative_path: path.to_owned(),
        digest_sha256: sha(digest_byte),
        file_kind: SurfaceFileKind::Regular,
        link_count: 1,
        status,
        active_readers: readers.iter().map(|value| (*value).to_owned()).collect(),
        active_writers: writers.iter().map(|value| (*value).to_owned()).collect(),
        public_routes: routes.iter().map(|value| (*value).to_owned()).collect(),
        generated_outputs: generated.iter().map(|value| (*value).to_owned()).collect(),
    })
}

pub(crate) fn inventory(surfaces: Vec<InventorySurface>, session: char) -> MigrationInventory {
    MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha(session), surfaces).unwrap()
}

pub(crate) fn false_pass_controls() -> Value {
    json!({
        "proof-artifact": sha('1'),
        "receipt-production": sha('2'),
        "score-only": sha('3'),
        "test-manipulation": sha('4'),
        "verbosity": sha('5')
    })
}

pub(crate) fn route(
    route_id: &str,
    source: &str,
    source_path: &str,
    target: &str,
    source_digest: char,
    target_digest: char,
    disposition: Option<&str>,
) -> Value {
    let (transition, adoption) = match disposition {
        Some("compatibility") => (
            json!({
                "compatibility_behavior":"exact-route-only",
                "compatibility_boundary":"explicit-only",
                "replacement_state":"verified",
                "active_reader_writer_state":"none",
                "observed_authority_state":"compatibility-route-retained",
                "equivalence_proof":"executed-behavior-v1",
                "physical_cleanup_state":"preserve",
                "proof_refs":[]
            }),
            Some(json!({
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"compatibility",
                "source_digest_sha256":sha(source_digest),
                "canonical_target_digest_sha256":sha(target_digest),
                "post_status":"context_only",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":[route_id],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":sha('6'),
                "rollback_execution_sha256":sha('7'),
                "false_pass_control_sha256":false_pass_controls(),
                "compatibility_prerequisites":{
                    "schema_version":"CompatibilityPrerequisites-v1",
                    "owner_id":"maintenance-owner",
                    "semantic_target_id":target,
                    "user_facing_warning":"This compatibility route is deprecated; use the canonical target.",
                    "usage_measurement":{
                        "schema_version":"CompatibilityUsageMeasurement-v1",
                        "route_id":route_id,
                        "metric":"legacy-route-invocations",
                        "evidence_sha256":sha('d'),
                        "window_start_unix_ms":1_000,
                        "window_end_unix_ms":2_000,
                        "observed_invocations":12
                    },
                    "boundary":{
                        "schema_version":"CompatibilityBoundary-v1",
                        "deadline_unix_ms":86_402_000
                    },
                    "removal_condition":{
                        "schema_version":"CompatibilityRemovalCondition-v1",
                        "metric":"legacy-route-invocations",
                        "operator":"less-than-or-equal",
                        "threshold":0,
                        "required_consecutive_windows":2
                    }
                },
                "preserve_physical_bytes":true
            })),
        ),
        Some("retirement") => (
            json!({
                "compatibility_behavior":"removed",
                "compatibility_boundary":"closed",
                "replacement_state":"verified",
                "active_reader_writer_state":"none",
                "observed_authority_state":"retired",
                "equivalence_proof":"executed-behavior-v1",
                "physical_cleanup_state":"preserve",
                "proof_refs":[]
            }),
            Some(json!({
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"retirement",
                "source_digest_sha256":sha(source_digest),
                "canonical_target_digest_sha256":sha(target_digest),
                "post_status":"retired",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":[],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":sha('8'),
                "rollback_execution_sha256":sha('9'),
                "false_pass_control_sha256":false_pass_controls(),
                "preserve_physical_bytes":true
            })),
        ),
        None => (
            json!({
                "compatibility_behavior":"unverified",
                "compatibility_boundary":"blocked-by-OD-008",
                "replacement_state":"unverified",
                "active_reader_writer_state":"active",
                "observed_authority_state":"active",
                "equivalence_proof":"missing",
                "physical_cleanup_state":"blocked-by-OD-009",
                "proof_refs":[]
            }),
            None,
        ),
        _ => unreachable!(),
    };
    let mut transition = transition;
    if let Some(adoption) = adoption {
        transition["adopted_effect"] = adoption;
    }
    json!({
        "route_id":route_id,
        "match":{"stable_id":source,"kind":"legacy-skill","relative_path":source_path},
        "canonical_target":target,
        "intended_disposition":"non-authoritative",
        "transition":transition
    })
}

pub(crate) fn registry_bytes(routes: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version":"AuthorityRoutingRegistry-v1",
        "contract_id":"harness-ultragoal-successor-contract-v2",
        "destructive_cleanup_authorized":false,
        "authority_rule":"Only exact adopted machine transitions may execute; prose and receipts are not retirement proof.",
        "routes":routes
    }))
    .unwrap()
}
