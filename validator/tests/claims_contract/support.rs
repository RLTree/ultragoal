use super::claims::{
    Actor, ActorRole, ClaimDefinitions, ClaimObligation, DecisionLedger, DecisionStatus,
    EvidenceEnvelope, EvidenceKind, LocalNegativeControlAuthority, ObligationKind,
    ObligationResult, Observation, SemanticControlObservation,
};
use super::context::{BuildRequest, LiveContext};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

struct TestBinding {
    context: LiveContext,
    scratch_root: PathBuf,
}

static TEST_BINDING: OnceLock<TestBinding> = OnceLock::new();
static CONTROL_MODELS: OnceLock<BTreeMap<String, BTreeMap<String, SemanticControlObservation>>> =
    OnceLock::new();

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

fn test_binding() -> &'static TestBinding {
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
    "../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json"
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

pub fn obligation_observation(
    definitions: &ClaimDefinitions,
    claim_id: &str,
    obligation: &ClaimObligation,
    ordinal: usize,
    tag: &str,
    false_pass_model: Option<SemanticControlObservation>,
) -> Observation {
    let definition = definitions.definition(claim_id).expect("definition");
    let evidence_id = format!("evidence-{claim_id}-{ordinal}-{tag}");
    assert_eq!(
        false_pass_model.is_some(),
        obligation.kind == ObligationKind::FalsePassControl,
        "false-pass semantic model must exactly match the obligation kind"
    );
    let result_digest = false_pass_model
        .as_ref()
        .map(|item| item.observed_digest().to_owned())
        .unwrap_or_else(|| digest(&format!("result:{evidence_id}")));
    let result = ObligationResult::Supported {
        result_digest: result_digest.clone(),
    };
    let (mut inputs, mut tools) = if let Some(observed) = &false_pass_model {
        let model = observed.model();
        (
            BTreeMap::from([(
                obligation.id.clone(),
                model.negative_stimulus_digest().to_owned(),
            )]),
            BTreeMap::from([(
                model.model_method().to_owned(),
                model.model_implementation_digest().to_owned(),
            )]),
        )
    } else {
        (
            BTreeMap::from([(
                "candidate-input".to_owned(),
                digest(&format!("input:{evidence_id}")),
            )]),
            BTreeMap::from([(
                "probe-tool".to_owned(),
                digest(&format!("tool:{evidence_id}")),
            )]),
        )
    };
    if matches!(
        obligation.kind,
        ObligationKind::RequiredSurface | ObligationKind::RequiredDecision
    ) {
        inputs.insert(
            obligation.id.clone(),
            digest(&format!("binding:{evidence_id}")),
        );
    }
    if obligation.kind == ObligationKind::RequiredTool {
        tools.insert(
            obligation.id.clone(),
            digest(&format!("tool-binding:{evidence_id}")),
        );
    }
    EvidenceEnvelope {
        evidence_id: evidence_id.clone(),
        claim_id: claim_id.to_owned(),
        obligation: obligation.clone(),
        producer: actor(
            &format!("producer-{claim_id}-{ordinal}-{tag}"),
            &[ActorRole::EvidenceProducer, ActorRole::IndependentObserver],
        ),
        method: format!("method-{claim_id}-{ordinal}-{tag}"),
        live_context_id: context_id().to_owned(),
        candidate_id: candidate_id().to_owned(),
        observed_at_unix_ms: now(),
        max_age_ms: 60_000,
        truth_surface: definition.truth_surface.clone(),
        declared_ceiling: definition.allowed_ceiling_on_pass.clone(),
        kind: EvidenceKind::DirectObservation,
        result,
        false_pass_model,
        inputs,
        environment_and_tools: tools,
        effects: BTreeMap::from([("read".to_owned(), "observed".to_owned())]),
        outputs: BTreeMap::from([(obligation.id.clone(), result_digest)]),
        artifact_digests: BTreeSet::from([digest(&format!("artifact:{evidence_id}"))]),
        limitations: vec![format!("bounded-to-{}", obligation.id)],
    }
    .observe()
    .expect("valid obligation observation")
}

pub fn observations_for(
    definitions: &ClaimDefinitions,
    claim_id: &str,
    tag: &str,
) -> Vec<Observation> {
    let mut models = if tag == "rerun" {
        authority(definitions)
            .model_claim_for_decision_tests(claim_id)
            .expect("rerun receives fresh named semantic control models")
    } else {
        cached_claim_controls(definitions, claim_id)
    };
    let observations = definitions
        .definition(claim_id)
        .expect("definition")
        .required_obligations()
        .iter()
        .enumerate()
        .map(|(index, obligation)| {
            let model = models.remove(&obligation.id);
            obligation_observation(definitions, claim_id, obligation, index, tag, model)
        })
        .collect();
    assert!(
        models.is_empty(),
        "all named semantic control models consumed exactly once"
    );
    observations
}

fn cached_claim_controls(
    definitions: &ClaimDefinitions,
    claim_id: &str,
) -> BTreeMap<String, SemanticControlObservation> {
    CONTROL_MODELS
        .get_or_init(|| {
            let bound = authority(definitions);
            definitions
                .order()
                .iter()
                .map(|claim_id| {
                    (
                        claim_id.clone(),
                        bound
                            .model_claim_for_decision_tests(claim_id)
                            .expect("authority models adopted named controls"),
                    )
                })
                .collect()
        })
        .get(claim_id)
        .cloned()
        .expect("cached adopted named controls")
}

pub fn semantic_model_ledger() -> DecisionLedger {
    DecisionLedger::for_semantic_model_tests()
}

pub fn submit(ledger: &mut DecisionLedger, observations: Vec<Observation>) -> Vec<String> {
    observations
        .into_iter()
        .map(|observation| {
            let id = observation.evidence_id().to_owned();
            ledger.submit(observation).expect("submit observation");
            id
        })
        .collect()
}

pub fn pass_claim(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    claim_id: &str,
    tag: &str,
) {
    let ids = submit(ledger, observations_for(definitions, claim_id, tag));
    let decision = ledger.decide(
        definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(definitions, claim_id),
        &ids,
    );
    assert_eq!(
        decision.status,
        DecisionStatus::Passed,
        "{claim_id}:{:?}",
        decision.reasons
    );
}

pub fn pass_before(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    target: &str,
    tag: &str,
) {
    for claim_id in definitions.order() {
        if claim_id == target {
            break;
        }
        pass_claim(ledger, definitions, claim_id, tag);
    }
}

pub fn digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn raw_digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
