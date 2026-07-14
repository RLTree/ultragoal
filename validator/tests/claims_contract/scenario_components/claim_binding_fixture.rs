use super::*;

pub(crate) struct TestBinding {
    pub(crate) context: LiveContext,
    pub(crate) scratch_root: PathBuf,
}

pub(crate) static TEST_BINDING: OnceLock<TestBinding> = OnceLock::new();

pub fn live_context() -> &'static LiveContext {
    &test_binding().context
}

pub fn context_id() -> &'static str {
    test_binding().context.context_id()
}

pub fn candidate_id() -> &'static str {
    static CANDIDATE: OnceLock<String> = OnceLock::new();
    CANDIDATE
        .get_or_init(|| {
            let bytes = serde_json::to_vec(live_context().candidate()).expect("candidate encode");
            digest_bytes(&bytes)
        })
        .as_str()
}

pub fn authority<'a>(definitions: &'a ClaimDefinitions) -> LocalNegativeControlAuthority<'a> {
    LocalNegativeControlAuthority::bind(definitions, live_context(), &test_binding().scratch_root)
        .expect("candidate-bound claim authority")
}

pub fn control_scratch_root() -> &'static Path {
    &test_binding().scratch_root
}

pub(crate) fn test_binding() -> &'static TestBinding {
    TEST_BINDING.get_or_init(|| {
        let base = Path::new("/tmp/hul-claim-control-builder-013")
            .join("runtime")
            .join(format!("claims-contract-{}", std::process::id()));
        let repository = base.join("candidate");
        let scratch_root = base.join("control-scratch");
        std::fs::create_dir_all(&repository).expect("create candidate repo");
        std::fs::create_dir_all(&scratch_root).expect("create control scratch");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&scratch_root, std::fs::Permissions::from_mode(0o700))
                .expect("private control scratch");
        }
        if !repository.join(".git").is_dir() {
            let status = Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&repository)
                .status()
                .expect("git init launches");
            assert!(status.success(), "git init succeeds");
        }
        let context =
            LiveContext::build(BuildRequest::new(&repository)).expect("build current live context");
        TestBinding {
            context,
            scratch_root,
        }
    })
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as u64
}
pub const REGISTRY: &str = include_str!(
    "../../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json"
);
pub const REGISTRY_SHA256: &str =
    "67e81c4eabe87d16d816a3d7dad352dc21a4a1994dd83b6582e9e4ba5eaa61bc";

pub fn definitions() -> ClaimDefinitions {
    ClaimDefinitions::adopt(REGISTRY.as_bytes(), REGISTRY_SHA256).expect("adopt registry")
}

pub fn actor(id: &str, roles: &[ActorRole]) -> Actor {
    Actor {
        actor_id: id.to_owned(),
        roles: roles.iter().cloned().collect(),
    }
}

pub fn reviewer(definitions: &ClaimDefinitions, claim_id: &str) -> Actor {
    actor(
        &definitions
            .definition(claim_id)
            .expect("definition")
            .independent_reconciler,
        &[ActorRole::IndependentReviewer],
    )
}

pub fn semantic_model_ledger() -> DecisionLedger {
    DecisionLedger::for_semantic_model_tests()
}
