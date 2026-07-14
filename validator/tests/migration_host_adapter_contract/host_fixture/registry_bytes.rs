pub(crate) fn registry_bytes(
    disposition: TestDisposition,
    source_digest: &str,
    target_digest: &str,
) -> Vec<u8> {
    let observed_now = now_ms();
    let transition = match disposition {
        TestDisposition::Retirement => json!({
            "compatibility_behavior":"removed",
            "compatibility_boundary":"closed",
            "replacement_state":"verified",
            "active_reader_writer_state":"none",
            "observed_authority_state":"retired",
            "equivalence_proof":"executed-behavior-v1",
            "physical_cleanup_state":"preserve",
            "proof_refs":[],
            "adopted_effect":{
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"retirement",
                "source_digest_sha256":source_digest,
                "canonical_target_digest_sha256":target_digest,
                "post_status":"retired",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":[],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":hash(b"retirement behavior proof"),
                "rollback_execution_sha256":hash(b"retirement rollback proof"),
                "false_pass_control_sha256":false_pass_controls(),
                "preserve_physical_bytes":true
            }
        }),
        TestDisposition::Compatibility => json!({
            "compatibility_behavior":"exact-route-only",
            "compatibility_boundary":"explicit-only",
            "replacement_state":"verified",
            "active_reader_writer_state":"none",
            "observed_authority_state":"compatibility-route-retained",
            "equivalence_proof":"executed-behavior-v1",
            "physical_cleanup_state":"preserve",
            "proof_refs":[],
            "adopted_effect":{
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"compatibility",
                "source_digest_sha256":source_digest,
                "canonical_target_digest_sha256":target_digest,
                "post_status":"context_only",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":["route-old-to-current"],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":hash(b"compatibility behavior proof"),
                "rollback_execution_sha256":hash(b"compatibility rollback proof"),
                "false_pass_control_sha256":false_pass_controls(),
                "compatibility_prerequisites":{
                    "schema_version":"CompatibilityPrerequisites-v1",
                    "owner_id":"maintenance-owner",
                    "semantic_target_id":"SKILL:current",
                    "user_facing_warning":"This synthetic compatibility route is deprecated.",
                    "usage_measurement":{
                        "schema_version":"CompatibilityUsageMeasurement-v1",
                        "route_id":"route-old-to-current",
                        "metric":"legacy-route-invocations",
                        "evidence_sha256":hash(b"synthetic usage evidence"),
                        "window_start_unix_ms":observed_now.saturating_sub(2_000),
                        "window_end_unix_ms":observed_now.saturating_sub(1_000),
                        "observed_invocations":1
                    },
                    "boundary":{
                        "schema_version":"CompatibilityBoundary-v1",
                        "deadline_unix_ms":observed_now.saturating_add(86_400_000)
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
            }
        }),
        TestDisposition::Pending => json!({
            "compatibility_behavior":"unverified",
            "compatibility_boundary":"blocked-by-OD-008",
            "replacement_state":"unverified",
            "active_reader_writer_state":"active",
            "observed_authority_state":"active",
            "equivalence_proof":"missing",
            "physical_cleanup_state":"blocked-by-OD-009",
            "proof_refs":[]
        }),
    };
    serde_json::to_vec(&json!({
        "schema_version":"AuthorityRoutingRegistry-v1",
        "contract_id":"harness-ultragoal-successor-contract-v2",
        "destructive_cleanup_authorized":false,
        "authority_rule":"Only exact adopted machine transitions may execute; prose and receipts are not retirement proof.",
        "routes":[{
            "route_id":"route-old-to-current",
            "match":{
                "stable_id":"LEGACY-SKILL:old",
                "kind":"legacy-skill",
                "relative_path":SOURCE_PATH
            },
            "canonical_target":"SKILL:current",
            "intended_disposition":"non-authoritative",
            "transition":transition
        }]
    }))
    .unwrap()
}

fn false_pass_controls() -> Value {
    json!({
        "proof-artifact":hash(b"false pass proof artifact"),
        "receipt-production":hash(b"false pass receipt production"),
        "score-only":hash(b"false pass score only"),
        "test-manipulation":hash(b"false pass test manipulation"),
        "verbosity":hash(b"false pass verbosity")
    })
}

pub(crate) fn write_new(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

pub(crate) fn overwrite(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub(crate) fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn create_directory(path: &Path, mode: u32) {
    fs::create_dir(path).unwrap();
    set_mode(path, mode);
}

fn unique_root() -> PathBuf {
    let mut random = [0_u8; 8];
    getrandom::fill(&mut random).unwrap();
    let suffix = u64::from_le_bytes(random);
    PathBuf::from(format!(
        "/private/tmp/hul-migration-host-contract-{}-{suffix:016x}",
        std::process::id()
    ))
}

fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}
