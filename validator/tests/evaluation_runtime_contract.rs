#![allow(dead_code)]

#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/evaluation/mod.rs"]
mod evaluation;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;

// Recreate the production-only capture dependency closure in this integration
// crate. The repository's legacy capture harness intentionally omits the
// scheduler adapter under cfg(test), so this focused contract imports the real
// adapter directly and supplies only its two-stream bounded-output test seam.
mod environment {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum InvocationSensitivity {
        Public,
        SecretBearing,
    }

    impl InvocationSensitivity {
        pub(super) fn from_bound_secrets(secrets: &[Vec<u8>]) -> Self {
            if secrets.iter().any(|secret| !secret.is_empty()) {
                Self::SecretBearing
            } else {
                Self::Public
            }
        }

        pub(super) const fn is_secret_bearing(self) -> bool {
            matches!(self, Self::SecretBearing)
        }
    }
}
mod output {
    use super::environment::InvocationSensitivity;
    use std::io::Read;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    pub(super) struct OutputBudget {
        limit: u64,
        observed: AtomicU64,
        exceeded: AtomicBool,
    }

    pub(super) struct PendingOutput(Vec<u8>);

    pub(super) struct CapturedOutput(Vec<u8>);

    impl CapturedOutput {
        pub(super) fn retained(&self) -> &[u8] {
            &self.0
        }
    }

    pub(super) struct StableOutputs {
        pub(super) first: CapturedOutput,
        pub(super) second: CapturedOutput,
        pub(super) output_limit_exceeded: bool,
    }

    impl OutputBudget {
        pub(super) fn for_sensitivity(limit: usize, _sensitivity: InvocationSensitivity) -> Self {
            Self {
                limit: limit as u64,
                observed: AtomicU64::new(0),
                exceeded: AtomicBool::new(false),
            }
        }

        fn claim(&self, requested: usize) -> usize {
            loop {
                let observed = self.observed.load(Ordering::SeqCst);
                let remaining = self.limit.saturating_sub(observed);
                let claimed = remaining.min(requested as u64);
                if self
                    .observed
                    .compare_exchange(
                        observed,
                        observed + claimed,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    if claimed < requested as u64 || claimed == 0 {
                        self.exceeded.store(true, Ordering::SeqCst);
                    }
                    return claimed as usize;
                }
            }
        }

        pub(super) fn exceeded(&self) -> bool {
            self.exceeded.load(Ordering::SeqCst)
        }

        pub(super) fn finalize_streams(
            &self,
            first: PendingOutput,
            second: PendingOutput,
        ) -> StableOutputs {
            StableOutputs {
                first: CapturedOutput(first.0),
                second: CapturedOutput(second.0),
                output_limit_exceeded: self.exceeded(),
            }
        }
    }

    pub(super) fn observe(
        mut reader: impl Read,
        _limit: usize,
        budget: &OutputBudget,
    ) -> Result<PendingOutput, String> {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|_| "captured output stream read failed".to_owned())?;
            if read == 0 {
                break;
            }
            let allowed = budget.claim(read);
            retained.extend_from_slice(&buffer[..allowed]);
            if allowed < read {
                break;
            }
        }
        Ok(PendingOutput(retained))
    }
}
#[path = "../src/cli/capture/fixture.rs"]
mod fixture_capture;

use evaluation::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionRuntimeError, execute_production,
};
use evaluation::{
    AdvisoryPractice, AuthorityAnalysis, BindingProductRequirement, BoundInput,
    ConfigurationExposure, EvaluationDataControls, EvaluationExecutionBinding,
    EvaluationLedgerState, EvaluationSpec, EvaluationTask, ExperimentalHypothesis,
    FactTemporalScope, FileEvaluationExecutionLedger, FilePromotionReviewLedger, ImpactAnalysis,
    InputKind, LawChangeProposal, MigrationAnalysis, PerturbationControl, PromotionLedgerBinding,
    PromotionLedgerState, PromotionReviewAuthority, ProofAnalysis, ProposalAnalyses,
    RejectedRecommendation, ResearchAudit, ResearchSource, ResearchSourceClass,
    ResearchSourceRecord, RuntimeConfiguration, VerifiedSourceFact,
};
use fixture_capture::FixtureCaptureAdapter;
use fixture_scheduler::{
    ConfinementPolicy, ExpectedOutcome, FixtureExecutionRecord, FixtureKind, FixtureScheduler,
    FixtureSpec, NetworkIsolation, ObservedOutcome, ResourceKind, RunDisposition,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn review_id(binding_sha256: &str, attestation_sha256: &str) -> String {
    digest(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes())
}

fn root(label: &str) -> PathBuf {
    let root = PathBuf::from("/private/tmp").join(format!(
        "hul-evaluation-runtime-086-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
}

fn fixture_spec(id: &str) -> FixtureSpec {
    FixtureSpec::new(
        id,
        FixtureKind::Positive,
        "evaluation-runtime-contract",
        BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
        ExpectedOutcome::pass(0),
        false,
    )
    .unwrap()
}

fn write_shell(path: &Path, output: &str) {
    fs::write(path, format!("#!/bin/sh\nprintf '%s' '{output}'\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o500)).unwrap();
}

fn wait_until(label: &str, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn process_barrier() -> Result<(), ()> {
    let Some(root) = std::env::var_os("HUL_EVAL_RACE_BARRIER") else {
        return Ok(());
    };
    let participant = std::env::var("HUL_EVAL_RACE_PARTICIPANT").map_err(|_| ())?;
    let root = PathBuf::from(root);
    fs::write(root.join(format!("ready-{participant}")), b"ready").map_err(|_| ())?;
    let release = root.join("release");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !release.is_file() {
        if Instant::now() >= deadline {
            return Err(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

fn release_process_barrier(root: &Path, participants: usize) {
    wait_until("process race participants", || {
        (0..participants).all(|index| root.join(format!("ready-{index}")).is_file())
    });
    fs::write(root.join("release"), b"release").unwrap();
}

#[test]
fn fixture_permit_rejects_issue_time_path_substitution() {
    let root = root("fixture-issue-swap");
    let original = root.join("fixture.sh");
    let accepted = root.join("accepted.sh");
    let replacement = root.join("replacement.sh");
    write_shell(&original, "accepted");
    write_shell(&replacement, "substituted");
    let fixture = fixture_spec("fixture-issue-swap");

    FixtureCaptureAdapter::set_test_issue_pause(original.clone(), 500);
    let issue_path = original.clone();
    let issuance = std::thread::spawn(move || {
        FixtureCaptureAdapter::issue(
            &fixture,
            issue_path,
            Vec::<OsString>::new(),
            4096,
            b"accepted".to_vec(),
        )
    });
    wait_until("fixture permit issue pause", || {
        FixtureCaptureAdapter::test_issue_is_paused()
    });
    fs::rename(&original, &accepted).unwrap();
    fs::rename(&replacement, &original).unwrap();
    let error = match issuance.join().unwrap() {
        Ok(_) => panic!("path substitution unexpectedly retained a fixture permit"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        fixture_scheduler::FixtureScheduleError::Integrity(_)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn captured_shell_bytes_survive_pre_launch_path_substitution() {
    let root = root("fixture-launch-swap");
    let source = root.join("fixture.sh");
    let accepted = root.join("accepted.sh");
    let replacement = root.join("replacement.sh");
    write_shell(&source, "accepted");
    write_shell(&replacement, "substituted");
    let fixture = fixture_spec("fixture-launch-swap");
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        source.clone(),
        Vec::<OsString>::new(),
        4096,
        b"accepted".to_vec(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease_id = scheduler.schedule([fixture]).unwrap().pop().unwrap();

    FixtureCaptureAdapter::set_test_pre_launch_pause(source.clone(), 500);
    let execution = std::thread::spawn(move || {
        scheduler
            .execute_recorded(&lease_id, &adapter)
            .map(|(disposition, record)| (disposition, record.artifact_bytes().to_vec()))
    });
    wait_until("fixture pre-launch pause", || {
        FixtureCaptureAdapter::test_pre_launch_is_paused()
    });
    fs::rename(&source, &accepted).unwrap();
    fs::rename(&replacement, &source).unwrap();

    let (disposition, artifact) = execution.join().unwrap().unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::Accepted);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(artifact, b"accepted");
    assert_ne!(artifact, b"substituted");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_shell_route_enforces_the_process_group_wall_timeout() {
    let root = root("fixture-shell-timeout");
    let source = root.join("fixture.sh");
    fs::write(&source, b"#!/bin/sh\nwhile :; do :; done\n").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o500)).unwrap();
    let fixture = FixtureSpec::new_with_confinement(
        "fixture-shell-timeout",
        FixtureKind::Negative,
        "evaluation-runtime-contract",
        BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Process]),
        ExpectedOutcome::causal_failure("fixture-wall-time-exceeded", 0),
        false,
        ConfinementPolicy {
            cpu_seconds: 5,
            address_space_bytes: 64 * 1024 * 1024,
            wall_time_millis: 100,
            maximum_file_bytes: 1024 * 1024,
            require_process_group: true,
            network: NetworkIsolation::DenyAll,
        },
    )
    .unwrap();
    let adapter =
        FixtureCaptureAdapter::issue(&fixture, source, Vec::<OsString>::new(), 4096, Vec::new())
            .unwrap();
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease_id = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    let started = Instant::now();
    let (disposition, record) = scheduler.execute_recorded(&lease_id, &adapter).unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::CausalFailure);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(record.exit_code, None);
    assert_eq!(record.outcome.causal_code, "fixture-wall-time-exceeded");
    assert_eq!(record.outcome.claim_ceiling, 0);
    assert!(started.elapsed() < Duration::from_secs(2));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_native_fixture_boundary_accepts_only_fixed_protected_substrates() {
    let root = root("fixture-native-boundary");
    let fixture = fixture_spec("fixture-native-boundary");
    FixtureCaptureAdapter::issue(
        &fixture,
        PathBuf::from("/usr/bin/true"),
        Vec::<OsString>::new(),
        4096,
        Vec::new(),
    )
    .unwrap();
    assert!(
        FixtureCaptureAdapter::issue(
            &fixture,
            PathBuf::from("/bin/sh"),
            vec![OsString::from("-c"), OsString::from("printf bypass")],
            4096,
            Vec::new(),
        )
        .is_err()
    );

    let copied = root.join("copied-native");
    fs::copy("/usr/bin/true", &copied).unwrap();
    fs::set_permissions(&copied, fs::Permissions::from_mode(0o500)).unwrap();
    assert!(
        FixtureCaptureAdapter::issue(&fixture, copied, Vec::<OsString>::new(), 4096, Vec::new(),)
            .is_err()
    );

    let linked = root.join("linked-native");
    std::os::unix::fs::symlink("/usr/bin/true", &linked).unwrap();
    assert!(
        FixtureCaptureAdapter::issue(&fixture, linked, Vec::<OsString>::new(), 4096, Vec::new(),)
            .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn task(id: &str, dataset: char) -> EvaluationTask {
    EvaluationTask::new(
        id,
        format!("REQ-{id}"),
        format!("behavior-{id}"),
        format!("fixture-{id}"),
        BoundInput::regular(format!("datasets/{id}.json"), sha(dataset), 128),
        format!("scorer-{id}"),
        sha('e'),
        controls(),
        true,
    )
}

fn spec(candidate: char) -> EvaluationSpec {
    EvaluationSpec::new(
        sha('a'),
        sha(candidate),
        "production-suite",
        vec![task("core", 'd')],
    )
    .unwrap()
}

struct FakeConfinedBridge;

impl FixtureEvaluationBridge for FakeConfinedBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError> {
        let artifact = serde_json::to_vec(&json!({
            "schema_version": "EvaluationFixtureArtifact-v1",
            "task_id": request.binding.task_id,
            "fixture_id": request.fixture_id,
            "outcome": "passed",
            "causal_code": "behavioral-pass",
            "score_earned": 10,
            "score_possible": 10,
            "work_units": 3,
            "producer_id": "fixture-producer",
            "observer_id": "fixture-observer",
            "independent_grader_id": "independent-grader",
            "independent_score_earned": 10,
            "independent_score_possible": 10,
            "passed_perturbations": [
                "verbosity",
                "proof_artifact",
                "receipt_production",
                "test_manipulation",
                "score_only"
            ]
        }))
        .unwrap();
        Ok(FixtureExecutionRecord::captured(
            request.binding.clone(),
            request.fixture_id.clone(),
            sha('f'),
            "lease-core".to_owned(),
            sha('1'),
            sha('2'),
            "result.json".to_owned(),
            artifact,
            ObservedOutcome::pass(0),
            Some(0),
            sha('3'),
            sha('4'),
            false,
        ))
    }
}

#[test]
fn audited_runtime_is_stable_artifact_bearing_and_explicitly_unknown() {
    let spec = spec('b');
    let audit = spec.audit(&sha('a'), &sha('b'));
    let mut first_bridge = FakeConfinedBridge;
    let first = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut first_bridge,
    )
    .unwrap();
    let mut second_bridge = FakeConfinedBridge;
    let second = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut second_bridge,
    )
    .unwrap();
    assert_eq!(first.canonical_run, second.canonical_run);
    assert_eq!(first.fixture_records().len(), 1);
    assert!(first.fixture_records()[0].artifact_byte_length > 0);
    assert_eq!(first.canonical_failures, Vec::new());
    assert_eq!(
        first.canonical_run.runtime_configuration.model,
        ConfigurationExposure::Unknown
    );
    assert_eq!(
        RuntimeConfiguration::from_prompt_text("model=gpt-from-prompt")
            .unwrap_err()
            .code(),
        "evaluation-prompt-derived-runtime-metadata-refused"
    );
}

#[test]
fn privacy_projection_has_no_claim_authority_or_secret_surface() {
    let spec = spec('b');
    let audit = spec.audit(&sha('a'), &sha('b'));
    let mut bridge = FakeConfinedBridge;
    let run = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut bridge,
    )
    .unwrap();
    let json = serde_json::to_string(&run.events)
        .unwrap()
        .to_ascii_lowercase();
    for forbidden in [
        "claim_id",
        "claim_authority",
        "readiness",
        "release",
        "acceptance",
        "completion",
        "api_key",
        "bearer ",
    ] {
        assert!(!json.contains(forbidden), "leaked event field {forbidden}");
    }
}

const RESEARCH_CHECKED_DAY_EPOCH: u64 = 1_783_900_800;
const RESEARCH_2026_01_01_EPOCH: u64 = 1_767_225_600;
const RESEARCH_MUTABLE_FRESHNESS_SECONDS: u64 = 30 * 86_400;

fn research_epoch(offset_seconds: u64) -> u64 {
    RESEARCH_CHECKED_DAY_EPOCH + offset_seconds
}

fn research_laws() -> BTreeSet<String> {
    BTreeSet::from(["HUL-RESEARCH-001".to_owned()])
}

fn research_source_url(source_id: &str) -> String {
    format!("https://primary.example/{source_id}")
}

fn research_record(
    source_id: &str,
    source_class: ResearchSourceClass,
    temporal_scope: FactTemporalScope,
    supported_proposals: BTreeSet<String>,
    observed_at_offset_seconds: u64,
) -> ResearchSourceRecord {
    let laws = research_laws();
    let url = research_source_url(source_id);
    complete_research_record(
        source_id,
        "Primary Example Publisher",
        source_class,
        "2026-07-13",
        research_epoch(observed_at_offset_seconds),
        vec![
            VerifiedSourceFact::new(
                "fact-current-capability",
                format!("Verified current capability fact from {source_id}."),
                source_id,
                format!("{url}#verified-fact"),
                laws.clone(),
                temporal_scope,
            )
            .unwrap(),
        ],
        supported_proposals,
    )
}

fn complete_research_record(
    source_id: &str,
    publisher: &str,
    source_class: ResearchSourceClass,
    checked_date: &str,
    observed_at_epoch_seconds: u64,
    verified_source_facts: Vec<VerifiedSourceFact>,
    supported_proposals: BTreeSet<String>,
) -> ResearchSourceRecord {
    let laws = research_laws();
    let url = research_source_url(source_id);
    ResearchSourceRecord::new(
        source_id,
        publisher,
        &url,
        source_class,
        checked_date,
        observed_at_epoch_seconds,
        verified_source_facts,
        vec![
            BindingProductRequirement::new(
                "REQ-RESEARCH-001",
                format!("Binding product requirement derived for {source_id}."),
                "HUL-RESEARCH-001",
            )
            .unwrap(),
        ],
        vec![
            AdvisoryPractice::new(
                "practice-bounded-review",
                format!("Advisory practice retained separately for {source_id}."),
                source_id,
                format!("{url}#advisory-practice"),
                laws.clone(),
            )
            .unwrap(),
        ],
        vec![
            ExperimentalHypothesis::new(
                "hypothesis-review-latency",
                format!("Experimental hypothesis recorded for {source_id}."),
                source_id,
                format!("{url}#experimental-hypothesis"),
                laws.clone(),
                "Reject when the representative latency sample exceeds the adopted bound.",
            )
            .unwrap(),
        ],
        vec![
            RejectedRecommendation::new(
                "recommendation-auto-adopt",
                format!("Rejected automatic-adoption recommendation from {source_id}."),
                source_id,
                format!("{url}#rejected-recommendation"),
                laws.clone(),
                "Only an explicit reviewed contract change may alter binding law.",
            )
            .unwrap(),
        ],
        vec!["The source does not prove installed or runtime product behavior.".to_owned()],
        laws,
        supported_proposals,
    )
    .unwrap()
}

fn bind_research_record(relative_path: &str, record: &ResearchSourceRecord) -> ResearchSource {
    let bytes = record.canonical_bytes();
    ResearchSource::test_only_from_root_adopted_record(
        BoundInput::regular(relative_path, digest(&bytes), bytes.len() as u64),
        bytes,
    )
    .unwrap()
}

fn bind_untrusted_research_record(
    relative_path: &str,
    record: &ResearchSourceRecord,
) -> ResearchSource {
    let bytes = record.canonical_bytes();
    ResearchSource::from_bound_record(
        BoundInput::regular(relative_path, digest(&bytes), bytes.len() as u64),
        bytes,
    )
    .unwrap()
}

fn research_source(
    source_id: &str,
    supported_proposals: BTreeSet<String>,
    observed_at_offset_seconds: u64,
) -> ResearchSource {
    let record = research_record(
        source_id,
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        supported_proposals,
        observed_at_offset_seconds,
    );
    bind_research_record(&format!("research/{source_id}.json"), &record)
}

fn audit_research(
    sources: &[ResearchSource],
    proposals: &[LawChangeProposal],
    current_offset_seconds: u64,
) -> ResearchAudit {
    ResearchAudit::test_only_audit_at(sources, proposals, research_epoch(current_offset_seconds))
}

fn proposal_analyses(
    supporting_sources: &BTreeSet<String>,
    mapped_laws: &BTreeSet<String>,
) -> ProposalAnalyses {
    ProposalAnalyses::new(
        ImpactAnalysis::new(
            "Impact is limited to the reviewed research contract surfaces.",
            BTreeSet::from(["REQ-RESEARCH-004".to_owned()]),
            BTreeSet::from(["research-law-change".to_owned()]),
            mapped_laws.clone(),
        )
        .unwrap(),
        MigrationAnalysis::new(
            "Migration requires a reviewed contract amendment before implementation.",
            vec!["Prepare and review a candidate-bound contract amendment.".to_owned()],
            vec![
                "Retain the prior binding law when review does not accept the amendment."
                    .to_owned(),
            ],
            mapped_laws.clone(),
        )
        .unwrap(),
        ProofAnalysis::new(
            "Proof must bind the reviewed amendment to current primary evidence.",
            BTreeSet::from(["REQ-RESEARCH-004-PROOF".to_owned()]),
            vec!["Substitute a source digest and require rejection.".to_owned()],
            mapped_laws.clone(),
        )
        .unwrap(),
        AuthorityAnalysis::root_review_required(
            "Only Ultra root may adopt a reviewed contract change.",
            supporting_sources.clone(),
            mapped_laws.clone(),
        )
        .unwrap(),
    )
}

fn research_proposal(proposal_id: &str, supporting_sources: BTreeSet<String>) -> LawChangeProposal {
    let mapped_laws = research_laws();
    LawChangeProposal::non_authoritative(
        proposal_id,
        format!("Proposal {proposal_id}"),
        format!("Current primary evidence supports review of {proposal_id}."),
        supporting_sources.clone(),
        mapped_laws.clone(),
        proposal_analyses(&supporting_sources, &mapped_laws),
    )
    .unwrap()
}

fn research_source_rejected(source: ResearchSource, expected_code: &str) {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from([source.source_id().to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == expected_code),
        "missing research finding {expected_code}: {:?}",
        audit.findings()
    );
}

fn research_untrusted_current_primary_rejected(source: ResearchSource) {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from([source.source_id().to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );
    for unexpected in [
        "research-source-invalid",
        "research-source-stable-fact-authority-required",
        "research-source-stale",
        "research-source-current-primary-required",
    ] {
        assert!(
            audit
                .findings()
                .iter()
                .all(|finding| finding.code() != unexpected),
            "untrusted current-primary control was rejected for {unexpected}: {:?}",
            audit.findings()
        );
    }
}

fn assert_only_no_authority_effects(value: &Value, effects: &mut usize) {
    match value {
        Value::Array(values) => {
            for value in values {
                assert_only_no_authority_effects(value, effects);
            }
        }
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "authority_effect" {
                    *effects += 1;
                    assert_eq!(value, "none");
                }
                assert_only_no_authority_effects(value, effects);
            }
        }
        _ => {}
    }
}

#[test]
fn research_current_supported_input_yields_only_a_non_authoritative_proposal() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let record = serde_json::to_value(source.record()).unwrap();
    for field in [
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
    ] {
        assert!(
            record.get(field).is_some(),
            "missing typed source field {field}"
        );
    }
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);

    assert!(audit.findings().is_empty());
    assert_eq!(audit.eligible_proposals().len(), 1);
    assert_eq!(audit.authority_effect(), "none");
    assert_eq!(audit.eligible_proposals()[0].authority_effect(), "none");

    let serialized = serde_json::to_value(&audit).unwrap();
    let mut effects = 0;
    assert_only_no_authority_effects(&serialized, &mut effects);
    assert_eq!(
        effects, 3,
        "audit, proposal, and authority analysis must each deny authority"
    );
}

#[test]
fn research_source_record_fields_and_canonical_binding_fail_closed() {
    let record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let bytes = record.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&bytes),
                bytes.len() as u64
            ),
            bytes.clone(),
        )
        .is_ok()
    );

    for required_field in [
        "schema_version",
        "source_id",
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "observed_at_epoch_seconds",
        "valid_until_epoch_seconds",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
        "supports_proposal_ids",
    ] {
        let mut value = serde_json::from_slice::<Value>(&bytes).unwrap();
        value.as_object_mut().unwrap().remove(required_field);
        let missing = serde_json::to_vec(&value).unwrap();
        assert!(
            ResearchSource::from_bound_record(
                BoundInput::regular(
                    "research/source-primary.json",
                    digest(&missing),
                    missing.len() as u64,
                ),
                missing,
            )
            .is_err(),
            "missing source field passed: {required_field}"
        );
    }
    for required_field in [
        "schema_version",
        "source_id",
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "observed_at_epoch_seconds",
        "valid_until_epoch_seconds",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
        "supports_proposal_ids",
    ] {
        let mut value = serde_json::from_slice::<Value>(&bytes).unwrap();
        value[required_field] = match required_field {
            "observed_at_epoch_seconds" | "valid_until_epoch_seconds" => json!(0),
            "verified_source_facts"
            | "binding_product_requirements"
            | "advisory_practices"
            | "experimental_hypotheses"
            | "rejected_recommendations"
            | "limitations"
            | "mapped_law_ids"
            | "supports_proposal_ids" => json!([]),
            _ => json!(""),
        };
        let empty = serde_json::to_vec(&value).unwrap();
        assert!(
            ResearchSource::from_bound_record(
                BoundInput::regular(
                    "research/source-primary.json",
                    digest(&empty),
                    empty.len() as u64,
                ),
                empty,
            )
            .is_err(),
            "empty source field passed: {required_field}"
        );
    }
    let mut unknown_value = serde_json::from_slice::<Value>(&bytes).unwrap();
    unknown_value["unknown_authority"] = json!("forbidden");
    let unknown = serde_json::to_vec(&unknown_value).unwrap();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&unknown),
                unknown.len() as u64,
            ),
            unknown,
        )
        .is_err(),
        "unknown source field passed"
    );

    let pretty = serde_json::to_vec_pretty(&record).unwrap();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&pretty),
                pretty.len() as u64,
            ),
            pretty,
        )
        .is_err(),
        "noncanonical source bytes passed"
    );
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular("research/source-primary.json", sha('f'), bytes.len() as u64),
            bytes.clone(),
        )
        .is_err(),
        "digest substitution passed"
    );
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-primary.json",
                digest(&bytes),
                bytes.len() as u64 + 1,
            ),
            bytes.clone(),
        )
        .is_err(),
        "length substitution passed"
    );
    for snapshot in [
        BoundInput::regular("../source.json", digest(&bytes), bytes.len() as u64),
        BoundInput::observed(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
            1,
            InputKind::Symlink,
        ),
        BoundInput::observed(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
            2,
            InputKind::Regular,
        ),
    ] {
        assert!(
            ResearchSource::from_bound_record(snapshot, bytes.clone()).is_err(),
            "unsafe source snapshot passed"
        );
    }
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular("research/source-primary.json", digest(&[]), 0),
            Vec::new(),
        )
        .is_err(),
        "empty source bytes passed"
    );

    let substituted_record = research_record(
        "source-substituted",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let forged = ResearchSource::test_only_unchecked(
        BoundInput::regular(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
        ),
        substituted_record,
        bytes,
    );
    research_source_rejected(forged, "research-source-invalid");
}

#[test]
fn research_source_substitution_mutate_restore_and_classification_laundering_fail_closed() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );

    let mut url_substitution = source.clone();
    url_substitution.substitute_url_for_test("https://substituted.example/source-primary");
    research_source_rejected(url_substitution, "research-source-invalid");

    let mut mutate_restore = source.clone();
    mutate_restore.substitute_url_for_test("https://substituted.example/source-primary");
    mutate_restore.substitute_url_for_test(research_source_url("source-primary"));
    research_source_rejected(mutate_restore, "research-source-invalid");

    let mut snapshot_substitution = source.clone();
    snapshot_substitution.substitute_snapshot_for_test(BoundInput::regular(
        "research/source-primary.json",
        sha('e'),
        source.record().canonical_bytes().len() as u64,
    ));
    research_source_rejected(snapshot_substitution, "research-source-invalid");

    let mut byte_substitution = source.clone();
    byte_substitution.substitute_record_bytes_for_test(source.record().canonical_bytes());
    research_source_rejected(byte_substitution, "research-source-invalid");

    for control in [
        "checked-date",
        "source-class",
        "limitation",
        "fact",
        "mapped-law",
    ] {
        let mut substituted = source.clone();
        substituted.substitute_typed_field_for_test(control);
        research_source_rejected(substituted, "research-source-invalid");
    }

    for control in [
        "fact-source-substitution",
        "fact-advice-laundering",
        "requirement-advice-laundering",
        "fact-hypothesis-laundering",
        "requirement-hypothesis-laundering",
    ] {
        let mut classified = source.clone();
        classified.invalidate_classification_for_test(control);
        research_source_rejected(classified, "research-source-invalid");
    }
}

#[test]
fn research_freshness_primary_source_and_support_mapping_fail_closed() {
    for current_epoch_seconds in [0, 9, RESEARCH_MUTABLE_FRESHNESS_SECONDS + 11] {
        let source = research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            10,
        );
        let proposal = research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        );
        let audit = audit_research(&[source], &[proposal], current_epoch_seconds);
        assert!(audit.eligible_proposals().is_empty());
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == "research-source-stale")
        );
    }

    let current_source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let boundary = audit_research(
        &[current_source.clone()],
        &[proposal.clone()],
        10 + RESEARCH_MUTABLE_FRESHNESS_SECONDS,
    );
    assert!(boundary.findings().is_empty());
    assert_eq!(boundary.eligible_proposals().len(), 1);
    let expired = audit_research(
        &[current_source.clone()],
        &[proposal],
        11 + RESEARCH_MUTABLE_FRESHNESS_SECONDS,
    );
    assert!(expired.eligible_proposals().is_empty());
    assert!(
        expired
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale")
    );

    for control in [
        "old-checked-date-current-observation",
        "source-declared-indefinite-validity",
    ] {
        let mut invalid = current_source.clone();
        invalid.invalidate_freshness_for_test(control);
        research_source_rejected(invalid, "research-source-invalid");
    }

    let mut non_primary = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    non_primary.substitute_class_for_test(ResearchSourceClass::VendorDocumentation);
    research_source_rejected(non_primary, "research-source-current-primary-required");

    let unsupported = research_source(
        "source-primary",
        BTreeSet::from(["proposal-other".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[unsupported], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-support-invalid")
    );
}

#[test]
fn research_external_stable_fact_and_rebound_laundering_fail_closed() {
    let source_id = "source-vendor-model-x";
    let source_url = research_source_url(source_id);
    let supported_proposals = BTreeSet::from(["proposal-safe".to_owned()]);
    let exact_counterexample = complete_research_record(
        source_id,
        "Model X Vendor",
        ResearchSourceClass::VendorDocumentation,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-model-x-current",
                "Model X is currently available in the product.",
                source_id,
                format!("{source_url}#model-x-current"),
                research_laws(),
                FactTemporalScope::Stable,
            )
            .unwrap(),
        ],
        supported_proposals.clone(),
    );
    let source = bind_untrusted_research_record(
        "research/source-vendor-model-x.json",
        &exact_counterexample,
    );
    let proposal = research_proposal("proposal-safe", BTreeSet::from([source_id.to_owned()]));
    let audit = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&source),
        std::slice::from_ref(&proposal),
        RESEARCH_CHECKED_DAY_EPOCH,
    );
    assert!(audit.eligible_proposals().is_empty());
    for expected in [
        "research-source-authority-binding-required",
        "research-source-stable-fact-authority-required",
        "research-source-stale",
        "research-source-current-primary-required",
    ] {
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == expected),
            "missing exact laundering finding {expected}: {:?}",
            audit.findings()
        );
    }
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );
    assert_eq!(audit.authority_effect(), "none");

    let current_record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        supported_proposals.clone(),
        10,
    );
    let externally_constructed_current =
        bind_untrusted_research_record("research/source-primary.json", &current_record);
    research_untrusted_current_primary_rejected(externally_constructed_current);

    let class_only_base = research_record(
        "source-class-only",
        ResearchSourceClass::VendorDocumentation,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let mut source_class_only_rebound = serde_json::to_value(class_only_base).unwrap();
    source_class_only_rebound["source_class"] = json!("primary_specification");
    let source_class_only_rebound: ResearchSourceRecord =
        serde_json::from_value(source_class_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-class-only.json",
        &source_class_only_rebound,
    ));

    let temporal_only_base = research_record(
        "source-temporal-only",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::Stable,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let mut temporal_only_rebound = serde_json::to_value(temporal_only_base).unwrap();
    temporal_only_rebound["verified_source_facts"][0]["temporal_scope"] =
        json!("mutable_capability");
    let temporal_only_rebound: ResearchSourceRecord =
        serde_json::from_value(temporal_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-temporal-only.json",
        &temporal_only_rebound,
    ));

    let observation_only_base = complete_research_record(
        "source-observation-only",
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-observation-only",
                "Model X was observed as available on the checked date.",
                "source-observation-only",
                format!(
                    "{}#observation-only",
                    research_source_url("source-observation-only")
                ),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
        ],
        BTreeSet::from(["proposal-safe".to_owned()]),
    );
    let mut observation_only_rebound = serde_json::to_value(observation_only_base).unwrap();
    observation_only_rebound["checked_date"] = json!("2026-07-13");
    observation_only_rebound["observed_at_epoch_seconds"] = json!(research_epoch(10));
    observation_only_rebound["valid_until_epoch_seconds"] =
        json!(research_epoch(10) + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let observation_only_rebound: ResearchSourceRecord =
        serde_json::from_value(observation_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-observation-only.json",
        &observation_only_rebound,
    ));

    let mut source_class_rebound = serde_json::to_value(&current_record).unwrap();
    source_class_rebound["source_class"] = json!("vendor_documentation");
    let source_class_rebound: ResearchSourceRecord =
        serde_json::from_value(source_class_rebound).unwrap();
    let source_class_rebound =
        bind_untrusted_research_record("research/source-primary.json", &source_class_rebound);
    research_source_rejected(
        source_class_rebound,
        "research-source-current-primary-required",
    );

    let mut label_rebound = serde_json::to_value(&current_record).unwrap();
    label_rebound["verified_source_facts"][0]["temporal_scope"] = json!("stable");
    let label_rebound: ResearchSourceRecord = serde_json::from_value(label_rebound).unwrap();
    let label_rebound =
        bind_untrusted_research_record("research/source-primary.json", &label_rebound);
    research_source_rejected(
        label_rebound,
        "research-source-stable-fact-authority-required",
    );

    let mixed_record = complete_research_record(
        "source-mixed",
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-07-13",
        research_epoch(10),
        vec![
            VerifiedSourceFact::new(
                "fact-current-capability",
                "Model X is currently available in the product.",
                "source-mixed",
                format!("{}#model-x-current", research_source_url("source-mixed")),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
            VerifiedSourceFact::new(
                "fact-attempted-stable",
                "The same caller also labels a second fact stable.",
                "source-mixed",
                format!("{}#attempted-stable", research_source_url("source-mixed")),
                research_laws(),
                FactTemporalScope::Stable,
            )
            .unwrap(),
        ],
        supported_proposals,
    );
    let mixed_source = bind_untrusted_research_record("research/source-mixed.json", &mixed_record);
    research_source_rejected(
        mixed_source,
        "research-source-stable-fact-authority-required",
    );
    let mixed_audit = audit_research(
        &[bind_untrusted_research_record(
            "research/source-mixed.json",
            &mixed_record,
        )],
        &[research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-mixed".to_owned()]),
        )],
        50,
    );
    assert!(
        mixed_audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );

    let mut self_consistent_substitution = serde_json::to_value(&exact_counterexample).unwrap();
    self_consistent_substitution["source_class"] = json!("primary_specification");
    self_consistent_substitution["verified_source_facts"][0]["temporal_scope"] =
        json!("mutable_capability");
    self_consistent_substitution["checked_date"] = json!("2026-07-13");
    self_consistent_substitution["observed_at_epoch_seconds"] = json!(research_epoch(10));
    self_consistent_substitution["valid_until_epoch_seconds"] =
        json!(research_epoch(10) + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let self_consistent_substitution: ResearchSourceRecord =
        serde_json::from_value(self_consistent_substitution).unwrap();
    let self_consistent_substitution = bind_untrusted_research_record(
        "research/source-vendor-model-x.json",
        &self_consistent_substitution,
    );
    assert_ne!(
        self_consistent_substitution.snapshot().digest_sha256(),
        source.snapshot().digest_sha256(),
        "combined rebound did not replace the canonical source digest"
    );
    research_untrusted_current_primary_rejected(self_consistent_substitution);

    let mut extended_validity = serde_json::to_value(&exact_counterexample).unwrap();
    extended_validity["valid_until_epoch_seconds"] =
        json!(RESEARCH_2026_01_01_EPOCH + 365 * 86_400);
    let extended_validity: ResearchSourceRecord =
        serde_json::from_value(extended_validity).unwrap();
    let extended_bytes = extended_validity.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-vendor-model-x.json",
                digest(&extended_bytes),
                extended_bytes.len() as u64,
            ),
            extended_bytes,
        )
        .is_err(),
        "self-consistent stable-validity extension passed after canonical-byte rebound"
    );
}

#[test]
fn research_production_clock_cannot_be_backdated_or_rebound_by_the_caller() {
    let source_id = "source-old-primary";
    let source_url = research_source_url(source_id);
    let old_record = complete_research_record(
        source_id,
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-old-capability",
                "Model X was observed as available on the checked date.",
                source_id,
                format!("{source_url}#old-capability"),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
        ],
        BTreeSet::from(["proposal-safe".to_owned()]),
    );
    let old_source = bind_research_record("research/source-old-primary.json", &old_record);
    let proposal = research_proposal("proposal-safe", BTreeSet::from([source_id.to_owned()]));

    let simulated_backdate = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
        RESEARCH_2026_01_01_EPOCH + 1,
    );
    assert_eq!(simulated_backdate.eligible_proposals().len(), 1);
    assert!(simulated_backdate.findings().is_empty());

    let production = ResearchAudit::audit(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
    );
    assert!(production.eligible_proposals().is_empty());
    assert!(
        production
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale"),
        "production clock accepted backdated source: {:?}",
        production.findings()
    );

    let future_now = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
        RESEARCH_CHECKED_DAY_EPOCH + 365 * 86_400,
    );
    assert!(future_now.eligible_proposals().is_empty());
    assert!(
        future_now
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale")
    );

    for invalid_now in [0, u64::MAX] {
        let audit = ResearchAudit::test_only_audit_at(
            std::slice::from_ref(&old_source),
            std::slice::from_ref(&proposal),
            invalid_now,
        );
        assert!(audit.eligible_proposals().is_empty());
        assert_eq!(audit.findings().len(), 1);
        assert_eq!(audit.findings()[0].code(), "research-clock-invalid");
        assert_eq!(audit.findings()[0].authority_effect(), "none");
    }

    let clock_failure = ResearchAudit::test_only_audit_with_clock_failure(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
    );
    assert!(clock_failure.eligible_proposals().is_empty());
    assert_eq!(clock_failure.findings().len(), 1);
    assert_eq!(
        clock_failure.findings()[0].code(),
        "research-clock-unavailable"
    );
    assert_eq!(clock_failure.findings()[0].authority_effect(), "none");

    let mut rebound = serde_json::to_value(&old_record).unwrap();
    rebound["valid_until_epoch_seconds"] =
        json!(RESEARCH_CHECKED_DAY_EPOCH + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let rebound: ResearchSourceRecord = serde_json::from_value(rebound).unwrap();
    let rebound_bytes = rebound.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-old-primary.json",
                digest(&rebound_bytes),
                rebound_bytes.len() as u64,
            ),
            rebound_bytes,
        )
        .is_err(),
        "same-record validity rebound passed after exact digest replacement"
    );
}

#[test]
fn research_incomplete_or_substituted_proposal_analyses_never_become_authority() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    for analysis in ["impact", "migration", "proof", "authority"] {
        let mut incomplete = proposal.clone();
        incomplete.test_only_clear_analysis(analysis);
        let audit = audit_research(&[source.clone()], &[incomplete], 50);
        assert!(audit.eligible_proposals().is_empty());
        assert!(audit.findings().iter().any(|finding| {
            finding.code() == "research-proposal-invalid" && finding.authority_effect() == "none"
        }));
        assert_eq!(audit.authority_effect(), "none");
    }
    for control in ["decision", "reviewer"] {
        let mut substituted = proposal.clone();
        substituted.test_only_substitute_authority(control);
        let audit = audit_research(&[source.clone()], &[substituted], 50);
        assert!(audit.eligible_proposals().is_empty());
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == "research-proposal-invalid")
        );
    }

    let mut source_substitution = proposal.clone();
    source_substitution
        .test_only_substitute_supporting_sources(BTreeSet::from(["source-substituted".to_owned()]));
    let audit = audit_research(&[source.clone()], &[source_substitution], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );

    let mut law_substitution = proposal;
    law_substitution.test_only_substitute_mapped_laws(BTreeSet::from(["HUL-OTHER-001".to_owned()]));
    let audit = audit_research(&[source], &[law_substitution], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );
}

#[test]
fn research_every_duplicate_source_and_proposal_participant_is_ineligible() {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let duplicate_id_sources = [
        research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            10,
        ),
        research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            11,
        ),
    ];
    let audit = audit_research(&duplicate_id_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-source-duplicate")
            .count(),
        2
    );

    let duplicate = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let duplicate_digest_sources = [duplicate.clone(), duplicate];
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&duplicate_digest_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-source-duplicate")
            .count(),
        2
    );

    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let duplicate_proposals = [
        research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        ),
        research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        ),
    ];
    let audit = audit_research(&[source], &duplicate_proposals, 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-proposal-duplicate")
            .count(),
        2
    );
}

#[test]
fn research_empty_and_oversized_sets_have_no_eligible_proposals() {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-count-out-of-bounds")
    );

    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let audit = audit_research(&[source.clone()], &[], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-count-out-of-bounds")
    );

    let oversized_sources = (0..129)
        .map(|index| {
            let source_id = format!("source-{index}");
            research_source(&source_id, BTreeSet::from(["proposal-safe".to_owned()]), 10)
        })
        .collect::<Vec<_>>();
    let proposal = research_proposal("proposal-safe", BTreeSet::from(["source-0".to_owned()]));
    let audit = audit_research(&oversized_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-count-out-of-bounds")
    );

    let oversized_proposals = (0..33)
        .map(|index| {
            research_proposal(
                &format!("proposal-{index}"),
                BTreeSet::from(["source-primary".to_owned()]),
            )
        })
        .collect::<Vec<_>>();
    let audit = audit_research(&[source], &oversized_proposals, 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-count-out-of-bounds")
    );
}

fn recursive_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_dir() {
                visit(root, &path, rows);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let bytes = if metadata.file_type().is_symlink() {
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec()
                } else {
                    fs::read(&path).unwrap()
                };
                rows.insert(relative, bytes);
            }
        }
    }

    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}

fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .unwrap();
    assert!(output.status.success(), "git status failed: {output:?}");
    output.stdout
}

#[test]
fn research_read_parse_query_and_audit_paths_are_recursively_zero_write() {
    let repository = root("research-zero-write");
    let initialized = Command::new("git")
        .args(["init", "-q"])
        .arg(&repository)
        .status()
        .unwrap();
    assert!(initialized.success());
    let research_dir = repository.join("research");
    fs::create_dir(&research_dir).unwrap();
    let record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::VersionClaim,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let source_path = research_dir.join("source-primary.json");
    fs::write(&source_path, record.canonical_bytes()).unwrap();
    let before_status = git_status(&repository);
    let before_tree = recursive_tree(&repository);

    let bytes = fs::read(&source_path).unwrap();
    let source = ResearchSource::from_bound_record(
        BoundInput::regular(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
        ),
        bytes,
    )
    .unwrap();
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );

    assert_eq!(git_status(&repository), before_status);
    assert_eq!(recursive_tree(&repository), before_tree);
    fs::remove_dir_all(repository).unwrap();
}

fn data_controls(
    success: &str,
    failure: &str,
    split: &str,
    training_splits: BTreeSet<String>,
    semantic: char,
    near: char,
    known_training: BTreeSet<String>,
    declared: &str,
    verified: &str,
    sampled: &str,
    target: &str,
) -> EvaluationDataControls {
    EvaluationDataControls::new(
        "observe-core",
        success,
        failure,
        split,
        training_splits,
        sha(semantic),
        sha(near),
        known_training,
        declared,
        verified,
        sampled,
        target,
    )
}

#[test]
fn ambiguity_split_near_duplicate_label_and_population_leakage_reject() {
    let cases = [
        (
            data_controls(
                "same",
                "same",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-task-ambiguous",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::from(["eval".to_owned()]),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-split-leakage-detected",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::from([sha('2')]),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-near-duplicate-leakage-detected",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "fail",
                "target",
                "target",
            ),
            "evaluation-label-mismatch",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "sample",
                "target",
            ),
            "evaluation-unrepresentative-data",
        ),
    ];
    for (controls, expected) in cases {
        let spec = EvaluationSpec::new(
            sha('a'),
            sha('b'),
            "leakage-suite",
            vec![task("core", 'd').with_data_controls(controls)],
        )
        .unwrap();
        assert!(
            spec.audit(&sha('a'), &sha('b'))
                .findings()
                .iter()
                .any(|finding| finding == expected),
            "missing {expected}"
        );
    }
}

fn execution_binding(candidate: char, session: char) -> EvaluationExecutionBinding {
    EvaluationExecutionBinding::new(
        sha('a'),
        sha(candidate),
        sha('c'),
        sha('d'),
        sha(session),
        sha('e'),
        sha('f'),
    )
    .unwrap()
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn execution_ledger_restart_recovery_and_read_paths_are_zero_write() {
    let root = root("execution-restart");
    let key = [7_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.require_recovery("publication-ambiguous").unwrap();
    drop(ledger);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    let before = tree(&root);
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    assert_eq!(tree(&root), before);
    reopened
        .reconcile_recovery(Some((sha('8'), sha('9'))))
        .unwrap();
    reopened.complete().unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_ledger_mutation_rollback_truncation_and_special_files_fail_closed() {
    let root = root("execution-malformed");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let _ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::write(root.join("execution.state"), b"{").unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding.clone()).is_err());
    fs::remove_file(root.join("execution.state")).unwrap();
    std::os::unix::fs::symlink("execution.anchor.journal", root.join("execution.state")).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore() {
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let state_only_root = root("execution-state-only-rollback");
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&state_only_root, key, binding.clone()).unwrap();
    let old_state = fs::read(state_only_root.join("execution.state")).unwrap();
    ledger.reserve().unwrap();
    fs::write(state_only_root.join("execution.state"), old_state).unwrap();
    let recovered =
        FileEvaluationExecutionLedger::open(&state_only_root, key, binding.clone()).unwrap();
    assert!(matches!(
        recovered.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));

    let paired_root = root("execution-paired-rollback");
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&paired_root, key, binding.clone()).unwrap();
    let old_state = fs::read(paired_root.join("execution.state")).unwrap();
    let old_anchor = fs::read(paired_root.join("execution.anchor.journal")).unwrap();
    ledger.reserve().unwrap();
    fs::write(paired_root.join("execution.state"), old_state).unwrap();
    fs::write(paired_root.join("execution.anchor.journal"), old_anchor).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&paired_root, key, binding).is_err());
    fs::remove_dir_all(state_only_root).unwrap();
    fs::remove_dir_all(paired_root).unwrap();
}

#[test]
fn execution_journal_tolerates_one_crash_tail_and_repairs_only_on_mutation() {
    let root = root("execution-partial-tail");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    drop(ledger);
    let anchor_path = root.join("execution.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&root);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding.clone()).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
    assert_eq!(tree(&root), before);
    reopened.reserve().unwrap();
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    let current = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        current.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_stale_protocol_pending_file_is_ignored_read_only_then_removed_on_mutation() {
    let root = root("execution-stale-pending");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    drop(ledger);
    let pending = root.join(".execution.state.pending.999999.1");
    fs::write(&pending, b"partial-publication").unwrap();
    fs::set_permissions(&pending, fs::Permissions::from_mode(0o600)).unwrap();

    let before = tree(&root);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));
    assert_eq!(tree(&root), before);
    reopened.require_recovery("pending-reconciled").unwrap();
    assert!(!pending.exists());
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_lock_replacement_cannot_create_a_second_mutation_authority() {
    let root = root("execution-lock-replacement");
    let saved_lock = root.with_extension("saved-execution-lock");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::rename(root.join("execution.lock"), &saved_lock).unwrap();
    let replacement = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("execution.lock"))
        .unwrap();
    drop(replacement);
    fs::set_permissions(
        root.join("execution.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(ledger.reserve().is_err());
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_file(root.join("execution.lock")).unwrap();
    fs::rename(&saved_lock, root.join("execution.lock")).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_final_named_root_revalidation_refuses_orphan_write_and_read_success() {
    let root = root("execution-final-root-swap");
    let saved = root.with_extension("orphaned-authority");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha('8'), sha('9')).unwrap();

    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let writer = std::thread::spawn(move || ledger.complete());
    wait_until("execution final write validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = writer.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-descriptor-substituted");
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();

    let terminal = FileEvaluationExecutionLedger::open(&root, key, binding.clone()).unwrap();
    assert!(matches!(
        terminal.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let reader = std::thread::spawn(move || terminal.terminal_proof().map(|_| ()));
    wait_until("execution final read validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = reader.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-descriptor-substituted");
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_final_named_lock_and_anchor_revalidation_refuses_late_swap_success() {
    for (label, component, expected_code) in [
        (
            "lock",
            "execution.lock",
            "evaluation-ledger-lock-descriptor-substituted",
        ),
        (
            "anchor",
            "execution.anchor.journal",
            "evaluation-anchor-descriptor-substituted",
        ),
    ] {
        let root = root(&format!("execution-final-{label}-swap"));
        let saved = root.with_extension(format!("saved-{label}"));
        let key = [8_u8; 32];
        let binding = execution_binding('b', '1');
        let mut ledger =
            FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
        ledger.reserve().unwrap();
        ledger.publish_result(sha('8'), sha('9')).unwrap();

        FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
        let writer = std::thread::spawn(move || ledger.complete());
        wait_until("execution component final validation", || {
            FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
        });
        fs::rename(root.join(component), &saved).unwrap();
        let replacement = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(component))
            .unwrap();
        drop(replacement);
        fs::set_permissions(root.join(component), fs::Permissions::from_mode(0o600)).unwrap();
        FileEvaluationExecutionLedger::release_test_final_validation(&root);
        let error = writer.join().unwrap().unwrap_err();
        assert_eq!(error.code(), expected_code);
        assert!(fs::read(root.join(component)).unwrap().is_empty());
        fs::remove_file(root.join(component)).unwrap();
        fs::rename(&saved, root.join(component)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn execution_final_named_state_revalidation_refuses_recoverable_late_swap_success() {
    let root = root("execution-final-state-swap");
    let saved = root.with_extension("terminal-state");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha('8'), sha('9')).unwrap();
    let published = fs::read(root.join("execution.state")).unwrap();

    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let writer = std::thread::spawn(move || ledger.complete());
    wait_until("execution state final validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(root.join("execution.state"), &saved).unwrap();
    fs::write(root.join("execution.state"), published).unwrap();
    fs::set_permissions(
        root.join("execution.state"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = writer.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-final-current-changed");
    fs::remove_file(root.join("execution.state")).unwrap();
    fs::rename(&saved, root.join("execution.state")).unwrap();

    let terminal = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        terminal.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

fn terminal_ledger(
    root: &Path,
    key: [u8; 32],
    candidate: char,
    session: char,
    run: char,
) -> FileEvaluationExecutionLedger {
    let binding = execution_binding(candidate, session);
    let mut ledger = FileEvaluationExecutionLedger::initialize(root, key, binding).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha(run), sha('9')).unwrap();
    ledger.complete().unwrap();
    ledger
}

fn promotion_binding(baseline_root: &Path, candidate_root: &Path) -> PromotionLedgerBinding {
    let baseline =
        FileEvaluationExecutionLedger::open(baseline_root, [1_u8; 32], execution_binding('b', '1'))
            .unwrap();
    let candidate = FileEvaluationExecutionLedger::open(
        candidate_root,
        [2_u8; 32],
        execution_binding('c', '2'),
    )
    .unwrap();
    PromotionLedgerBinding::from_terminal_proofs(
        "promotion-authority",
        "independent-reviewer",
        sha('3'),
        baseline.terminal_proof().unwrap(),
        candidate.terminal_proof().unwrap(),
    )
    .unwrap()
}

#[test]
fn execution_and_promotion_roots_reject_symlinked_ancestors() {
    let parent = root("ledger-symlink-ancestor");
    let alias = parent.with_extension("ancestor-alias");
    let execution_root = parent.join("execution");
    let review_root = parent.join("review");
    fs::create_dir(&execution_root).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&execution_root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&parent, &alias).unwrap();

    let execution_error = FileEvaluationExecutionLedger::initialize(
        alias.join("execution"),
        [7_u8; 32],
        execution_binding('b', '1'),
    )
    .err()
    .unwrap();
    assert_eq!(execution_error.code(), "evaluation-ledger-root-open-failed");

    let baseline_root = root("symlink-ancestor-baseline");
    let candidate_root = root("symlink-ancestor-candidate");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let promotion_error =
        FilePromotionReviewLedger::initialize(alias.join("review"), [3_u8; 32], binding)
            .err()
            .unwrap();
    assert_eq!(promotion_error.code(), "promotion-ledger-root-unsafe");

    fs::remove_file(alias).unwrap();
    fs::remove_dir_all(parent).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
}

#[test]
fn execution_descriptor_scan_cannot_be_redirected_away_from_an_unknown_entry() {
    let root = root("execution-descriptor-directory-scan");
    let saved = root.with_extension("descriptor-authority");
    let key = [7_u8; 32];
    let binding = execution_binding('b', '1');
    let ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::write(root.join("unrecognized-authority"), b"must be observed").unwrap();

    FileEvaluationExecutionLedger::set_test_directory_scan_pause(root.clone(), 10_000);
    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let reader = std::thread::spawn(move || ledger.inspect());
    wait_until("execution descriptor directory scan", || {
        FileEvaluationExecutionLedger::test_directory_scan_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_directory_scan(&root);
    wait_until("execution descriptor scan rejection", || {
        reader.is_finished()
            || FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    let reached_final_validation =
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root);
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = reader.join().unwrap().unwrap_err();
    assert!(
        !reached_final_validation,
        "a path-based scan reached final validation before observing the descriptor-root entry"
    );
    assert_eq!(error.code(), "evaluation-ledger-unknown-or-pending-entry");
    fs::remove_file(root.join("unrecognized-authority")).unwrap();
    let reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn promotion_descriptor_scan_cannot_be_redirected_away_from_an_unknown_entry() {
    let baseline_root = root("promotion-scan-baseline");
    let candidate_root = root("promotion-scan-candidate");
    let review_root = root("promotion-descriptor-directory-scan");
    let saved = review_root.with_extension("descriptor-authority");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let review =
        FilePromotionReviewLedger::initialize(&review_root, [3_u8; 32], binding.clone()).unwrap();
    fs::write(
        review_root.join("unrecognized-authority"),
        b"must be observed",
    )
    .unwrap();

    FilePromotionReviewLedger::set_test_directory_scan_pause(review_root.clone(), 10_000);
    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let reader = std::thread::spawn(move || review.inspect());
    wait_until("promotion descriptor directory scan", || {
        FilePromotionReviewLedger::test_directory_scan_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_directory_scan(&review_root);
    wait_until("promotion descriptor scan rejection", || {
        reader.is_finished()
            || FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    let reached_final_validation =
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root);
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    let error = reader.join().unwrap().unwrap_err();
    assert!(
        !reached_final_validation,
        "a path-based scan reached final validation before observing the descriptor-root entry"
    );
    assert_eq!(error.code(), "promotion-ledger-unknown-or-pending-entry");
    fs::remove_file(review_root.join("unrecognized-authority")).unwrap();
    let reopened = FilePromotionReviewLedger::open(&review_root, [3_u8; 32], binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Ready
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn distinct_review_ledger_is_one_shot_and_rejects_replay() {
    let baseline_root = root("promotion-baseline");
    let candidate_root = root("promotion-candidate");
    let review_root = root("promotion-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, [3_u8; 32], binding).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    assert!(review.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ));
    assert!(!review.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ));
    assert!(matches!(
        review.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore() {
    let baseline_root = root("promotion-rollback-baseline");
    let candidate_root = root("promotion-rollback-candidate");
    let state_only_root = root("promotion-state-only-rollback");
    let paired_root = root("promotion-paired-rollback");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];

    let mut review =
        FilePromotionReviewLedger::initialize(&state_only_root, key, binding.clone()).unwrap();
    let old_state = fs::read(state_only_root.join("promotion-review.state")).unwrap();
    review.issue_attestation(&sha('6')).unwrap();
    fs::write(state_only_root.join("promotion-review.state"), old_state).unwrap();
    let recovered =
        FilePromotionReviewLedger::open(&state_only_root, key, binding.clone()).unwrap();
    assert!(matches!(
        recovered.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));

    let mut review =
        FilePromotionReviewLedger::initialize(&paired_root, key, binding.clone()).unwrap();
    let old_state = fs::read(paired_root.join("promotion-review.state")).unwrap();
    let old_anchor = fs::read(paired_root.join("promotion-review.anchor.journal")).unwrap();
    review.issue_attestation(&sha('6')).unwrap();
    fs::write(paired_root.join("promotion-review.state"), old_state).unwrap();
    fs::write(
        paired_root.join("promotion-review.anchor.journal"),
        old_anchor,
    )
    .unwrap();
    assert!(FilePromotionReviewLedger::open(&paired_root, key, binding).is_err());

    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(state_only_root).unwrap();
    fs::remove_dir_all(paired_root).unwrap();
}

#[test]
fn promotion_journal_tolerates_one_crash_tail_and_repairs_only_on_mutation() {
    let baseline_root = root("promotion-partial-baseline");
    let candidate_root = root("promotion-partial-candidate");
    let review_root = root("promotion-partial-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let review = FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    drop(review);
    let anchor_path = review_root.join("promotion-review.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Ready
    ));
    assert_eq!(tree(&review_root), before);
    reopened.issue_attestation(&sha('6')).unwrap();
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_idempotent_issue_repairs_a_crash_tail_with_a_fresh_authenticated_generation() {
    let baseline_root = root("promotion-idempotent-tail-baseline");
    let candidate_root = root("promotion-idempotent-tail-candidate");
    let review_root = root("promotion-idempotent-tail-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let binding_sha256 = sha('6');
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    drop(review);

    let anchor_path = review_root.join("promotion-review.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding.clone()).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    assert_eq!(tree(&review_root), before);
    assert_eq!(
        reopened.issue_attestation(&binding_sha256).unwrap(),
        attestation
    );
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    let current = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        current.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));

    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_stale_protocol_pending_file_is_ignored_read_only_then_removed_on_mutation() {
    let baseline_root = root("promotion-pending-baseline");
    let candidate_root = root("promotion-pending-candidate");
    let review_root = root("promotion-pending-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    drop(review);
    let pending = review_root.join(".promotion-review.state.pending.999999.1");
    fs::write(&pending, b"partial-publication").unwrap();
    fs::set_permissions(&pending, fs::Permissions::from_mode(0o600)).unwrap();

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    assert_eq!(tree(&review_root), before);
    let review_id = review_id(&binding_sha256, &attestation);
    assert!(reopened.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ));
    assert!(!pending.exists());
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_lock_replacement_cannot_create_a_second_mutation_authority() {
    let baseline_root = root("promotion-lock-baseline");
    let candidate_root = root("promotion-lock-candidate");
    let review_root = root("promotion-lock-replacement");
    let saved_lock = review_root.with_extension("saved-promotion-lock");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    fs::rename(review_root.join("promotion-review.lock"), &saved_lock).unwrap();
    let replacement = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(review_root.join("promotion-review.lock"))
        .unwrap();
    drop(replacement);
    fs::set_permissions(
        review_root.join("promotion-review.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(review.require_recovery("lock-replaced").is_err());
    assert!(FilePromotionReviewLedger::open(&review_root, key, binding).is_err());
    fs::remove_file(review_root.join("promotion-review.lock")).unwrap();
    fs::rename(&saved_lock, review_root.join("promotion-review.lock")).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_final_named_root_revalidation_refuses_orphan_consume_and_read_success() {
    let baseline_root = root("promotion-final-baseline");
    let candidate_root = root("promotion-final-candidate");
    let review_root = root("promotion-final-root-swap");
    let saved = review_root.with_extension("orphaned-authority");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);

    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let writer = std::thread::spawn(move || {
        review.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion final write validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(!writer.join().unwrap());
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();

    let consumed = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        consumed.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let reader = std::thread::spawn(move || consumed.inspect());
    wait_until("promotion final read validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(reader.join().unwrap().is_err());
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_final_named_lock_and_anchor_revalidation_refuses_late_swap_success() {
    for (label, component) in [
        ("lock", "promotion-review.lock"),
        ("anchor", "promotion-review.anchor.journal"),
    ] {
        let baseline_root = root(&format!("promotion-final-{label}-baseline"));
        let candidate_root = root(&format!("promotion-final-{label}-candidate"));
        let review_root = root(&format!("promotion-final-{label}-swap"));
        let saved = review_root.with_extension(format!("saved-{label}"));
        let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
        let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
        let binding = promotion_binding(&baseline_root, &candidate_root);
        let key = [3_u8; 32];
        let mut review =
            FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
        let binding_sha256 = sha('6');
        let attestation = review.issue_attestation(&binding_sha256).unwrap();
        let review_id = review_id(&binding_sha256, &attestation);

        FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
        let writer = std::thread::spawn(move || {
            review.verify_and_consume(
                &binding_sha256,
                "independent-reviewer",
                &review_id,
                &attestation,
            )
        });
        wait_until("promotion component final validation", || {
            FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
        });
        fs::rename(review_root.join(component), &saved).unwrap();
        let replacement = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(review_root.join(component))
            .unwrap();
        drop(replacement);
        fs::set_permissions(
            review_root.join(component),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        FilePromotionReviewLedger::release_test_final_validation(&review_root);
        assert!(!writer.join().unwrap());
        assert!(fs::read(review_root.join(component)).unwrap().is_empty());
        fs::remove_file(review_root.join(component)).unwrap();
        fs::rename(&saved, review_root.join(component)).unwrap();
        fs::remove_dir_all(baseline_root).unwrap();
        fs::remove_dir_all(candidate_root).unwrap();
        fs::remove_dir_all(review_root).unwrap();
    }
}

#[test]
fn promotion_final_named_state_revalidation_refuses_recoverable_late_swap_success() {
    let baseline_root = root("promotion-final-state-baseline");
    let candidate_root = root("promotion-final-state-candidate");
    let review_root = root("promotion-final-state-swap");
    let saved = review_root.with_extension("consumed-state");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    let issued = fs::read(review_root.join("promotion-review.state")).unwrap();

    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let writer = std::thread::spawn(move || {
        review.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion state final validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(review_root.join("promotion-review.state"), &saved).unwrap();
    fs::write(review_root.join("promotion-review.state"), issued).unwrap();
    fs::set_permissions(
        review_root.join("promotion-review.state"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(!writer.join().unwrap());
    fs::remove_file(review_root.join("promotion-review.state")).unwrap();
    fs::rename(&saved, review_root.join("promotion-review.state")).unwrap();

    let consumed = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        consumed.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn execution_readers_wait_for_a_consistent_publication_pair() {
    let root = root("execution-publication-pair");
    let key = [4_u8; 32];
    let binding = execution_binding('b', '1');
    let mut writer =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    FileEvaluationExecutionLedger::set_test_publication_pause(root.clone(), 5_000);
    let writer_thread = std::thread::spawn(move || writer.reserve());
    wait_until("execution state-anchor publication pause", || {
        FileEvaluationExecutionLedger::test_publication_is_paused(&root)
    });

    let (sender, receiver) = mpsc::channel();
    let readers = (0..8)
        .map(|_| {
            let sender = sender.clone();
            let root = root.clone();
            let binding = binding.clone();
            std::thread::spawn(move || {
                let result = FileEvaluationExecutionLedger::open(root, key, binding)
                    .and_then(|ledger| ledger.inspect());
                sender.send(result).unwrap();
            })
        })
        .collect::<Vec<_>>();
    drop(sender);
    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
    FileEvaluationExecutionLedger::release_test_publication(&root);
    writer_thread.join().unwrap().unwrap();
    let states = receiver.into_iter().collect::<Vec<_>>();
    assert_eq!(states.len(), 8);
    assert!(
        states
            .into_iter()
            .all(|state| { matches!(state, Ok(EvaluationLedgerState::Reserved)) })
    );
    for reader in readers {
        reader.join().unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn promotion_readers_wait_for_a_consistent_publication_pair() {
    let baseline_root = root("promotion-pair-baseline");
    let candidate_root = root("promotion-pair-candidate");
    let review_root = root("promotion-pair-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [5_u8; 32];
    let mut writer =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = writer.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    FilePromotionReviewLedger::set_test_publication_pause(review_root.clone(), 5_000);
    let writer_thread = std::thread::spawn(move || {
        writer.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion state-anchor publication pause", || {
        FilePromotionReviewLedger::test_publication_is_paused(&review_root)
    });

    let (sender, receiver) = mpsc::channel();
    let readers = (0..8)
        .map(|_| {
            let sender = sender.clone();
            let review_root = review_root.clone();
            let binding = binding.clone();
            std::thread::spawn(move || {
                let result = FilePromotionReviewLedger::open(review_root, key, binding)
                    .and_then(|ledger| ledger.inspect());
                sender.send(result).unwrap();
            })
        })
        .collect::<Vec<_>>();
    drop(sender);
    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
    FilePromotionReviewLedger::release_test_publication(&review_root);
    assert!(writer_thread.join().unwrap());
    let states = receiver.into_iter().collect::<Vec<_>>();
    assert_eq!(states.len(), 8);
    assert!(
        states
            .into_iter()
            .all(|state| { matches!(state, Ok(PromotionLedgerState::Consumed { .. })) })
    );
    for reader in readers {
        reader.join().unwrap();
    }
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn two_process_execution_worker() {
    let Some(root) = std::env::var_os("HUL_EVAL_EXECUTION_RACE_ROOT") else {
        return;
    };
    if process_barrier().is_err() {
        std::process::exit(82);
    }
    match FileEvaluationExecutionLedger::open(root, [4_u8; 32], execution_binding('b', '1'))
        .and_then(|mut ledger| ledger.reserve())
    {
        Ok(()) => std::process::exit(80),
        Err(error) if error.code() == "evaluation-execution-reservation-conflict" => {
            std::process::exit(81)
        }
        Err(error) => {
            eprintln!("unexpected execution race result: {}", error.code());
            std::process::exit(82)
        }
    }
}

#[test]
fn execution_reservation_has_exactly_one_two_process_winner() {
    let ledger_root = root("execution-race");
    let ledger = FileEvaluationExecutionLedger::initialize(
        &ledger_root,
        [4_u8; 32],
        execution_binding('b', '1'),
    )
    .unwrap();
    drop(ledger);
    let exe = std::env::current_exe().unwrap();
    let barrier = root("execution-race-barrier");
    let mut children = (0..2)
        .map(|index| {
            Command::new(&exe)
                .args(["--exact", "two_process_execution_worker", "--nocapture"])
                .env("HUL_EVAL_EXECUTION_RACE_ROOT", &ledger_root)
                .env("HUL_EVAL_RACE_BARRIER", &barrier)
                .env("HUL_EVAL_RACE_PARTICIPANT", index.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release_process_barrier(&barrier, 2);
    let mut statuses = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    statuses.sort_unstable();
    assert_eq!(statuses, vec![80, 81]);
    fs::remove_dir_all(ledger_root).unwrap();
    fs::remove_dir_all(barrier).unwrap();
}

#[test]
fn two_process_review_worker() {
    let Some(review_root) = std::env::var_os("HUL_EVAL_REVIEW_RACE_ROOT") else {
        return;
    };
    if process_barrier().is_err() {
        std::process::exit(85);
    }
    let baseline_root = PathBuf::from(std::env::var_os("HUL_EVAL_BASELINE_ROOT").unwrap());
    let candidate_root = PathBuf::from(std::env::var_os("HUL_EVAL_CANDIDATE_ROOT").unwrap());
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut ledger = match FilePromotionReviewLedger::open(review_root, [5_u8; 32], binding) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("unexpected review race open result: {}", error.code());
            std::process::exit(85)
        }
    };
    let binding_sha256 = sha('6');
    let attestation = std::env::var("HUL_EVAL_ATTESTATION").unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    if ledger.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ) {
        std::process::exit(83);
    }
    match ledger.inspect() {
        Ok(PromotionLedgerState::Consumed { .. }) => std::process::exit(84),
        Ok(state) => {
            eprintln!("unexpected review race loser state: {state:?}");
            std::process::exit(85)
        }
        Err(error) => {
            eprintln!("unexpected review race inspect result: {}", error.code());
            std::process::exit(85)
        }
    }
}

#[test]
fn review_consumption_has_exactly_one_two_process_winner() {
    let baseline_root = root("review-race-baseline");
    let candidate_root = root("review-race-candidate");
    let review_root = root("review-race");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut ledger =
        FilePromotionReviewLedger::initialize(&review_root, [5_u8; 32], binding).unwrap();
    let attestation = ledger.issue_attestation(&sha('6')).unwrap();
    drop(ledger);
    let exe = std::env::current_exe().unwrap();
    let barrier = root("review-race-barrier");
    let mut children = (0..2)
        .map(|index| {
            Command::new(&exe)
                .args(["--exact", "two_process_review_worker", "--nocapture"])
                .env("HUL_EVAL_REVIEW_RACE_ROOT", &review_root)
                .env("HUL_EVAL_BASELINE_ROOT", &baseline_root)
                .env("HUL_EVAL_CANDIDATE_ROOT", &candidate_root)
                .env("HUL_EVAL_ATTESTATION", &attestation)
                .env("HUL_EVAL_RACE_BARRIER", &barrier)
                .env("HUL_EVAL_RACE_PARTICIPANT", index.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release_process_barrier(&barrier, 2);
    let mut statuses = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    statuses.sort_unstable();
    assert_eq!(statuses, vec![83, 84]);
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
    fs::remove_dir_all(barrier).unwrap();
}

#[test]
fn evaluation_worker_result_is_exact_typed_and_self_excluded() {
    const RESULT_PATH: &str =
        "docs/ultragoal-successor-live/worker-results/EVALUATION-PRODUCTION-RUNTIME-086.json";
    const ARTIFACT_PATHS: [&str; 18] = [
        "fixtures/evaluation-engine/paired-valid.json",
        "fixtures/evaluation-engine/red-cases.json",
        "validator/src/cli/capture/fixture.rs",
        "validator/src/cli/capture/fixture/execute.rs",
        "validator/src/cli/capture/fixture/permit.rs",
        "validator/src/evaluation/ledger.rs",
        "validator/src/evaluation/mod.rs",
        "validator/src/evaluation/promotion_ledger.rs",
        "validator/src/evaluation/records.rs",
        "validator/src/evaluation/research.rs",
        "validator/src/evaluation/runtime.rs",
        "validator/src/fixture_scheduler/mod.rs",
        "validator/src/fixture_scheduler/outcome.rs",
        "validator/src/fixture_scheduler/scheduler.rs",
        "validator/tests/evaluation_contract.rs",
        "validator/tests/evaluation_runtime_contract.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/detached_descendant.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/process_group.rs",
    ];
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let result_bytes = fs::read(repository.join(RESULT_PATH)).unwrap();
    let _typed = ultragoal::orchestration::WorkerResultV1::parse_json(&result_bytes).unwrap();
    let result: Value = serde_json::from_slice(&result_bytes).unwrap();
    let object = result.as_object().unwrap();
    let expected_keys = BTreeSet::from([
        "artifacts",
        "base_state",
        "candidate_identity",
        "changes",
        "commands_and_tests",
        "context_id",
        "dependency_nodes",
        "effects",
        "final_state",
        "findings",
        "fixtures",
        "generated_outputs",
        "lease_id",
        "limitations",
        "no_claim_statement",
        "requested_root_changes",
        "requirements",
        "touched_paths",
        "touched_semantics",
        "unresolved_dependencies",
        "worker",
    ]);
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected_keys
    );
    assert_eq!(
        result["worker"],
        "/root/evaluation_research_authority_repair_engineer"
    );
    assert_eq!(
        result["lease_id"],
        "EVALUATION-RESEARCH-AUTHORITY-BINDING-086-R7"
    );
    assert_eq!(
        result["no_claim_statement"],
        "This worker does not claim readiness, release, or completion."
    );
    assert_eq!(
        result["final_state"]["status"],
        "candidate_for_root_acceptance"
    );
    let lease_exact_ceiling = "This worker does not claim root adoption, public evaluation command availability, installed runtime behavior, representative product journeys, claim elevation, readiness, release, or completion.";
    assert_eq!(
        result["final_state"]["lease_exact_no_claim_statement"],
        lease_exact_ceiling
    );
    assert!(
        result["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation == lease_exact_ceiling)
    );
    assert_eq!(result["generated_outputs"], json!([RESULT_PATH]));
    assert_eq!(
        result["fixtures"],
        json!([
            "fixtures/evaluation-engine/paired-valid.json",
            "fixtures/evaluation-engine/red-cases.json"
        ])
    );

    let mut expected_touched = ARTIFACT_PATHS.to_vec();
    expected_touched.push(RESULT_PATH);
    assert_eq!(result["touched_paths"], json!(expected_touched));
    let rows = result["artifacts"].as_array().unwrap();
    assert_eq!(rows.len(), ARTIFACT_PATHS.len());
    let mut aggregate = Sha256::new();
    let mut aggregate_bytes = 0_u64;
    for (row, path) in rows.iter().zip(ARTIFACT_PATHS) {
        assert_eq!(row["path"], path);
        let full = repository.join(path);
        let metadata = fs::symlink_metadata(&full).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.nlink(), 1);
        let bytes = fs::read(full).unwrap();
        let file_sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
        assert_eq!(row["sha256"], file_sha256);
        assert_eq!(row["byte_length"], bytes.len() as u64);
        aggregate.update(path.as_bytes());
        aggregate.update(b"\t");
        aggregate.update(file_sha256.trim_start_matches("sha256:").as_bytes());
        aggregate.update(b"\n");
        aggregate_bytes += bytes.len() as u64;
    }
    let aggregate = format!("sha256:{:x}", aggregate.finalize());
    assert_eq!(
        result["candidate_identity"]["artifact_count"],
        ARTIFACT_PATHS.len() as u64
    );
    assert_eq!(
        result["candidate_identity"]["artifact_bytes"],
        aggregate_bytes
    );
    assert_eq!(
        result["candidate_identity"]["artifact_set_sha256"],
        aggregate
    );
    assert_eq!(result["candidate_identity"]["candidate_id"], aggregate);
    assert_eq!(result["final_state"]["source_candidate_id"], aggregate);
}
