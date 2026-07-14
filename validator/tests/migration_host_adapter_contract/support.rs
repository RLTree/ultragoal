use crate::migration::product::{
    DarwinMigrationAdapters, DarwinMigrationHost, ProductMigrationPlan, derive_product_plan,
    provision_darwin_migration_host_for_test,
};
use crate::migration::{InventorySurface, MigrationInventory, SurfaceFileKind, SurfaceStatus};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const PRODUCT_VERSION: &str = "0.0.12";
pub(crate) const SOURCE_PATH: &str = "skills/old/SKILL.md";
pub(crate) const TARGET_PATH: &str = "skills/current/SKILL.md";
pub(crate) const REGISTRY_PATH: &str = "migration/authority-routes.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestDisposition {
    Retirement,
    Compatibility,
    Pending,
}

pub(crate) struct TestHost {
    pub(crate) root: PathBuf,
    pub(crate) repository: PathBuf,
    pub(crate) state: PathBuf,
    pub(crate) inventory: MigrationInventory,
    pub(crate) candidate_id: String,
    source_bytes: Vec<u8>,
}

impl TestHost {
    pub(crate) fn new(disposition: TestDisposition) -> Self {
        let root = unique_root();
        let repository = root.join("repository");
        let state = root.join("state");
        create_directory(&root, 0o700);
        create_directory(&repository, 0o700);
        create_directory(&repository.join("skills"), 0o700);
        create_directory(&repository.join("skills/old"), 0o700);
        create_directory(&repository.join("skills/current"), 0o700);
        create_directory(&repository.join("migration"), 0o700);

        let source_bytes = b"synthetic legacy migration source\n".to_vec();
        let target_bytes = b"synthetic canonical migration target\n".to_vec();
        write_new(&repository.join(SOURCE_PATH), &source_bytes, 0o600);
        write_new(&repository.join(TARGET_PATH), &target_bytes, 0o600);

        let source_digest = hash(&source_bytes);
        let target_digest = hash(&target_bytes);
        let registry = registry_bytes(disposition, &source_digest, &target_digest);
        write_new(&repository.join(REGISTRY_PATH), &registry, 0o600);

        let candidate_id = hash(b"synthetic migration candidate");
        let source_status = match disposition {
            TestDisposition::Retirement | TestDisposition::Pending => SurfaceStatus::Candidate,
            TestDisposition::Compatibility => SurfaceStatus::Active,
        };
        let source = InventorySurface::observed(
            "LEGACY-SKILL:old",
            "legacy-skill",
            SOURCE_PATH,
            source_digest,
            SurfaceFileKind::Regular,
            1,
            source_status,
            vec!["legacy-reader".to_owned()],
            vec!["legacy-writer".to_owned()],
            vec!["legacy-public".to_owned()],
            vec!["legacy-generated".to_owned()],
        );
        let target = InventorySurface::observed(
            "SKILL:current",
            "skill",
            TARGET_PATH,
            target_digest,
            SurfaceFileKind::Regular,
            1,
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec!["current-public".to_owned()],
            vec![],
        );
        let inventory = MigrationInventory::new(
            hash(b"synthetic live context"),
            candidate_id.clone(),
            hash(b"synthetic catalog"),
            hash(b"synthetic read session"),
            vec![source, target],
        )
        .unwrap();
        provision_darwin_migration_host_for_test(&repository, &state, &inventory, PRODUCT_VERSION)
            .unwrap();
        Self {
            root,
            repository,
            state,
            inventory,
            candidate_id,
            source_bytes,
        }
    }

    pub(crate) fn open(
        &self,
    ) -> Result<DarwinMigrationAdapters, crate::migration::product::HostError> {
        DarwinMigrationHost::open(
            &self.repository,
            &self.state,
            self.inventory.clone(),
            &self.candidate_id,
            PRODUCT_VERSION,
        )
    }

    pub(crate) fn derive_plan(
        &self,
        adapters: &mut DarwinMigrationAdapters,
    ) -> ProductMigrationPlan {
        use crate::migration::product::MigrationInputSource;
        let input = adapters.source.capture().unwrap();
        let authority = adapters.boundary_authority().unwrap();
        derive_product_plan(&input, Some(&authority)).unwrap()
    }

    pub(crate) fn state_bytes(&self) -> Vec<u8> {
        fs::read(self.state.join("state.json")).unwrap()
    }

    pub(crate) fn source_bytes(&self) -> Vec<u8> {
        fs::read(self.repository.join(SOURCE_PATH)).unwrap()
    }

    pub(crate) fn original_source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        if self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("hul-migration-host-contract-"))
        {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

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
