use super::definition::{ADOPTED_CLAIM_REGISTRY_SHA256, ClaimDefinition, ClaimDefinitions};
use super::evidence::{Actor, ActorRole};
pub use super::false_pass_receipt::{ExecutionObservation, ExecutionOutcome, FalsePassExecution};
use crate::context::LiveContext;
use crate::fixture_scheduler::{
    ConfinementPlan, ExpectedOutcome, FixtureExecutor, FixtureKind, FixtureScheduleError,
    FixtureScheduler, FixtureSpec, IsolationLease, ObservedOutcome, ResourceKind, RunDisposition,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub(super) const CONTROL_RUNNER: &str = "/bin/sh";
pub(super) const EXECUTION_METHOD: &str = "fixture-scheduler-named-control-v2";
pub(super) const OBSERVATION_METHOD: &str = "claim-authority-observation-v2";
const CONTROL_CATALOG_VERSION: &str = "adopted-claim-negative-controls-v1";
const CONTROL_SCRIPT_VERSION: &str = "closed-shell-control-v1";
pub(super) const EXPECTED_EXIT_CODE: i32 = 70;
const CONTROL_MAX_AGE_MS: u64 = 600_000;
const CONTROL_MAX_OUTPUT_BYTES: usize = 4 * 1024;
static EXECUTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Unforgeable within the claims module: sibling evidence producers can name
/// this type but cannot construct its private field.
pub(super) struct ReceiptAuthority {
    _private: (),
}

impl ReceiptAuthority {
    fn issue_for_test_transport() -> Self {
        Self { _private: () }
    }
}

/// Candidate binding is issued only from a revalidated `LiveContext`.
///
/// Production execution remains fail-closed until root adopts exact upstream
/// product-control executors. The test-only transport below exercises receipt,
/// confinement, substitution, and replay mechanics; it is never product proof.
pub struct LocalNegativeControlAuthority<'a> {
    definitions: &'a ClaimDefinitions,
    context: LiveContext,
    context_id: String,
    candidate_id: String,
    scratch_root: PathBuf,
}

impl<'a> LocalNegativeControlAuthority<'a> {
    pub(crate) fn bind(
        definitions: &'a ClaimDefinitions,
        context: &LiveContext,
        scratch_root: impl AsRef<Path>,
    ) -> Result<Self, String> {
        if definitions.registry_digest() != ADOPTED_CLAIM_REGISTRY_SHA256 {
            return Err("claims-control-registry-not-adopted".to_owned());
        }
        context
            .revalidate()
            .map_err(|_| "claims-control-live-context-invalid".to_owned())?;
        let context_id = context.context_id().to_owned();
        let candidate_id = candidate_id(context)?;
        if !digest(&context_id) || !digest(&candidate_id) {
            return Err("claims-control-candidate-binding-invalid".to_owned());
        }
        let scratch_root = validate_scratch_root(scratch_root.as_ref())?;
        validate_registry_control_index(definitions)?;
        Ok(Self {
            definitions,
            context: context.clone(),
            context_id,
            candidate_id,
            scratch_root,
        })
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn execute_claim(
        &self,
        claim_id: &str,
    ) -> Result<BTreeMap<String, ExecutionObservation>, String> {
        #[cfg(not(test))]
        {
            let _ = claim_id;
            return product_executor_catalog_preflight();
        }
        #[cfg(test)]
        {
            self.context
                .revalidate()
                .map_err(|_| "claims-control-live-context-invalid".to_owned())?;
            let definition = self
                .definitions
                .definition(claim_id)
                .ok_or_else(|| "claims-control-unknown-claim".to_owned())?;
            let mut observations = BTreeMap::new();
            for (ordinal, control_id) in definition.false_pass_controls.iter().enumerate() {
                let control = NamedControlDefinition::derive(
                    self.definitions.registry_digest(),
                    definition,
                    control_id,
                    ordinal,
                )?;
                let observation = self.execute_control(&control, None)?;
                if observations
                    .insert(control_id.clone(), observation)
                    .is_some()
                {
                    return Err("claims-control-catalog-duplicate".to_owned());
                }
            }
            self.context
                .revalidate()
                .map_err(|_| "claims-control-candidate-drift".to_owned())?;
            if self.context.context_id() != self.context_id
                || candidate_id(&self.context)? != self.candidate_id
            {
                return Err("claims-control-candidate-drift".to_owned());
            }
            Ok(observations)
        }
    }

    fn execute_control(
        &self,
        control: &NamedControlDefinition,
        substitution: Option<TestSubstitution>,
    ) -> Result<ExecutionObservation, String> {
        let sequence = EXECUTION_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
        let issued_at = now_unix_ms()?;
        let nonce = digest_value(&format!(
            "claim-control-nonce-v1:{}:{}:{}:{}:{}:{}",
            control.definition_digest,
            self.context_id,
            self.candidate_id,
            std::process::id(),
            sequence,
            issued_at
        ));
        let plan =
            ControlExecutionPlan::issue(control, &self.context_id, &self.candidate_id, &nonce)?;
        let batch_root = self
            .scratch_root
            .join(format!("batch-{}-{sequence}", std::process::id()));
        let fixture = plan.fixture()?;
        let mut adapter = NamedControlAdapter::issue(&fixture, &plan)?;
        #[cfg(test)]
        if let Some(substitution) = substitution {
            match substitution {
                TestSubstitution::GenericTool => {
                    adapter.executable = PathBuf::from("/usr/bin/false");
                }
                TestSubstitution::Arguments => {
                    adapter.arguments.push(OsString::from("substituted"));
                }
                TestSubstitution::FixtureSpec => {
                    let mut substituted = fixture.clone();
                    substituted.semantic_target.push_str("-substituted");
                    return run_substituted_fixture(&batch_root, substituted, &adapter);
                }
            }
        }
        let mut scheduler = FixtureScheduler::new(&batch_root);
        let lease_id = scheduler
            .schedule([fixture])
            .map_err(schedule_error)?
            .pop()
            .ok_or_else(|| "claims-control-fixture-not-scheduled".to_owned())?;
        let disposition = match scheduler.execute(&lease_id, &adapter) {
            Ok(disposition) => disposition,
            Err(error) => {
                let _ = scheduler.recover(&lease_id);
                let _ = fs::remove_dir(&batch_root);
                return Err(schedule_error(error));
            }
        };
        if disposition != RunDisposition::CausalFailure {
            let _ = fs::remove_dir(&batch_root);
            return Err("claims-control-causal-failure-not-observed".to_owned());
        }
        let capture = match adapter.take_capture()? {
            Some(capture) => capture,
            None => {
                let _ = fs::remove_dir(&batch_root);
                return Err("claims-control-process-capture-missing".to_owned());
            }
        };
        fs::remove_dir(&batch_root).map_err(|_| "claims-control-scratch-cleanup-failed")?;
        let receipt_authority = ReceiptAuthority::issue_for_test_transport();
        let execution = FalsePassExecution::from_authority(
            &receipt_authority,
            digest_value(&format!(
                "claim-control-execution-v2:{}",
                capture.process_digest
            )),
            self.definitions.registry_digest().to_owned(),
            control.claim_id.clone(),
            control.control_id.clone(),
            control.definition_digest.clone(),
            plan.command_spec_digest.clone(),
            plan.argument_digest,
            plan.script_digest,
            nonce,
            plan.negative_stimulus_digest,
            control.expected_failure_contract.clone(),
            Actor {
                actor_id: format!(
                    "claim-control-executor:{}",
                    short_digest(&control.definition_digest)
                ),
                roles: BTreeSet::from([ActorRole::ControlExecutor]),
            },
            format!("{EXECUTION_METHOD}:{sequence}"),
            plan.runner_digest,
            capture.started_at_unix_ms,
            capture.ended_at_unix_ms,
            self.context_id.clone(),
            self.candidate_id.clone(),
            CONTROL_MAX_AGE_MS,
            control.truth_surface.clone(),
            control.declared_ceiling.clone(),
            capture.exit_code,
            ExecutionOutcome::ExecutedFailure {
                failure_code: control.expected_failure_contract.clone(),
                outcome_digest: capture.process_digest.clone(),
            },
            BTreeMap::from([(control.control_id.clone(), capture.process_digest.clone())]),
            BTreeSet::from([digest_value(&format!(
                "claim-control-artifact-v2:{}:{}",
                plan.command_spec_digest, capture.process_digest
            ))]),
        );
        ExecutionObservation::from_authority(
            &receipt_authority,
            execution,
            Actor {
                actor_id: format!(
                    "claim-control-observer:{}",
                    short_digest(&control.definition_digest)
                ),
                roles: BTreeSet::from([
                    ActorRole::ExecutionObserver,
                    ActorRole::IndependentObserver,
                ]),
            },
            format!("{OBSERVATION_METHOD}:{sequence}"),
            now_unix_ms()?.max(capture.ended_at_unix_ms),
            capture.process_digest,
        )
    }

    #[cfg(test)]
    pub(crate) fn execute_substituted_for_test(
        &self,
        claim_id: &str,
        control_id: &str,
        substitution: TestSubstitution,
    ) -> Result<ExecutionObservation, String> {
        let definition = self
            .definitions
            .definition(claim_id)
            .ok_or_else(|| "claims-control-unknown-claim".to_owned())?;
        let ordinal = definition
            .false_pass_controls
            .iter()
            .position(|candidate| candidate == control_id)
            .ok_or_else(|| "claims-control-not-required-by-claim".to_owned())?;
        let control = NamedControlDefinition::derive(
            self.definitions.registry_digest(),
            definition,
            control_id,
            ordinal,
        )?;
        self.execute_control(&control, Some(substitution))
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestSubstitution {
    GenericTool,
    Arguments,
    FixtureSpec,
}

#[cfg(not(test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestSubstitution {}

fn product_executor_catalog_preflight<T>() -> Result<T, String> {
    Err("claims-control-product-executor-catalog-unavailable".to_owned())
}

#[cfg(test)]
pub(crate) fn product_executor_catalog_preflight_for_test() -> Result<(), String> {
    product_executor_catalog_preflight()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NamedControlDefinition {
    pub(super) registry_digest: String,
    pub(super) claim_id: String,
    pub(super) control_id: String,
    pub(super) control_ordinal: usize,
    pub(super) truth_surface: String,
    pub(super) declared_ceiling: String,
    pub(super) expected_failure_contract: String,
    pub(super) fixture_id: String,
    pub(super) definition_digest: String,
    pub(super) runner_digest: String,
}

impl NamedControlDefinition {
    pub(super) fn derive(
        registry_digest: &str,
        definition: &ClaimDefinition,
        control_id: &str,
        control_ordinal: usize,
    ) -> Result<Self, String> {
        if registry_digest != ADOPTED_CLAIM_REGISTRY_SHA256
            || definition
                .false_pass_controls
                .get(control_ordinal)
                .map(String::as_str)
                != Some(control_id)
        {
            return Err("claims-control-definition-not-adopted".to_owned());
        }
        let runner_digest = protected_executable_digest(Path::new(CONTROL_RUNNER))?;
        let base = serde_json::to_vec(&(
            CONTROL_CATALOG_VERSION,
            registry_digest,
            &definition.claim_id,
            control_id,
            control_ordinal,
            &definition.truth_surface,
            &definition.allowed_ceiling_on_pass,
            CONTROL_RUNNER,
            &runner_digest,
            CONTROL_SCRIPT_VERSION,
            EXPECTED_EXIT_CODE,
        ))
        .map_err(|_| "claims-control-definition-encode-failed".to_owned())?;
        let definition_digest = digest_bytes(&base);
        let short = short_digest(&definition_digest);
        let claim_slug = definition
            .claim_id
            .trim_start_matches("CL-")
            .to_ascii_lowercase();
        Ok(Self {
            registry_digest: registry_digest.to_owned(),
            claim_id: definition.claim_id.clone(),
            control_id: control_id.to_owned(),
            control_ordinal,
            truth_surface: definition.truth_surface.clone(),
            declared_ceiling: definition.allowed_ceiling_on_pass.clone(),
            expected_failure_contract: format!(
                "claims-negative-control-rejected:{claim_slug}:{control_ordinal}:{short}"
            ),
            fixture_id: format!("hul-claim-{claim_slug}-{control_ordinal}-{short}"),
            definition_digest,
            runner_digest,
        })
    }
}

pub(super) fn expected_control_definition(
    registry_digest: &str,
    definition: &ClaimDefinition,
    control_id: &str,
) -> Result<NamedControlDefinition, String> {
    let ordinal = definition
        .false_pass_controls
        .iter()
        .position(|candidate| candidate == control_id)
        .ok_or_else(|| "claims-control-not-required-by-claim".to_owned())?;
    NamedControlDefinition::derive(registry_digest, definition, control_id, ordinal)
}

pub(super) fn validate_named_control_execution(
    registry_digest: &str,
    definition: &ClaimDefinition,
    execution: &FalsePassExecution,
) -> Result<(), String> {
    let expected =
        expected_control_definition(registry_digest, definition, execution.control_id())?;
    let plan = ControlExecutionPlan::issue(
        &expected,
        execution.live_context_id(),
        execution.candidate_id(),
        execution.authority_nonce(),
    )?;
    let expected_executor = format!(
        "claim-control-executor:{}",
        short_digest(&expected.definition_digest)
    );
    let expected_output = BTreeMap::from([(
        expected.control_id.clone(),
        execution
            .actual_causal_outcome()
            .outcome_digest()
            .to_owned(),
    )]);
    let expected_artifact = BTreeSet::from([digest_value(&format!(
        "claim-control-artifact-v2:{}:{}",
        plan.command_spec_digest,
        execution.actual_causal_outcome().outcome_digest()
    ))]);
    let expected_execution_id = digest_value(&format!(
        "claim-control-execution-v2:{}",
        execution.actual_causal_outcome().outcome_digest()
    ));
    if execution.registry_digest() != registry_digest
        || execution.claim_id() != definition.claim_id
        || execution.control_id() != expected.control_id
        || execution.control_definition_digest() != expected.definition_digest
        || execution.command_spec_digest() != plan.command_spec_digest
        || execution.argument_digest() != plan.argument_digest
        || execution.script_digest() != plan.script_digest
        || execution.negative_stimulus_digest() != plan.negative_stimulus_digest
        || execution.expected_failure_contract() != expected.expected_failure_contract
        || execution.executor_tool_digest() != expected.runner_digest
        || execution.executor().actor_id != expected_executor
        || execution.truth_surface() != expected.truth_surface
        || execution.declared_ceiling() != expected.declared_ceiling
        || execution.max_age_ms() != CONTROL_MAX_AGE_MS
        || execution.exit_code() != EXPECTED_EXIT_CODE
        || execution.execution_id() != expected_execution_id
        || execution.output_digests() != &expected_output
        || execution.artifact_digests() != &expected_artifact
        || !execution
            .actual_causal_outcome()
            .is_expected_failure(&expected.expected_failure_contract)
        || method_sequence(execution.execution_method(), EXECUTION_METHOD).is_none()
    {
        return Err("claims-control-execution-definition-mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn expected_observer_id(
    registry_digest: &str,
    definition: &ClaimDefinition,
    control_id: &str,
) -> Result<String, String> {
    let expected = expected_control_definition(registry_digest, definition, control_id)?;
    Ok(format!(
        "claim-control-observer:{}",
        short_digest(&expected.definition_digest)
    ))
}

pub(super) fn method_sequence<'a>(method: &'a str, prefix: &str) -> Option<u64> {
    method
        .strip_prefix(prefix)
        .and_then(|suffix| suffix.strip_prefix(':'))
        .and_then(|suffix| suffix.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn validate_registry_control_index(definitions: &ClaimDefinitions) -> Result<(), String> {
    let mut control_bindings = BTreeSet::new();
    for claim_id in definitions.order() {
        let definition = definitions
            .definition(claim_id)
            .ok_or_else(|| "claims-control-catalog-unknown-claim".to_owned())?;
        for (ordinal, control_id) in definition.false_pass_controls.iter().enumerate() {
            let _ = ordinal;
            if !control_bindings.insert((claim_id.clone(), control_id.clone())) {
                return Err("claims-control-catalog-conflict".to_owned());
            }
        }
    }
    if control_bindings.len()
        != definitions
            .order()
            .iter()
            .map(|claim_id| {
                definitions
                    .definition(claim_id)
                    .map(|definition| definition.false_pass_controls.len())
                    .unwrap_or(0)
            })
            .sum::<usize>()
    {
        return Err("claims-control-catalog-incomplete".to_owned());
    }
    Ok(())
}

struct ControlExecutionPlan {
    control: NamedControlDefinition,
    arguments: Vec<OsString>,
    expected_stdout: Vec<u8>,
    argument_digest: String,
    script_digest: String,
    command_spec_digest: String,
    negative_stimulus_digest: String,
    runner_digest: String,
}

impl ControlExecutionPlan {
    fn issue(
        control: &NamedControlDefinition,
        context_id: &str,
        candidate_id: &str,
        nonce: &str,
    ) -> Result<Self, String> {
        if !digest(context_id) || !digest(candidate_id) || !digest(nonce) {
            return Err("claims-control-binding-not-digest".to_owned());
        }
        let script = format!(
            "set -eu\n[ \"$#\" -eq 4 ] || exit 91\n[ \"$1\" = '{definition}' ] || exit 92\n[ \"$2\" = '{context}' ] || exit 93\n[ \"$3\" = '{candidate}' ] || exit 94\n[ \"$4\" = '{nonce}' ] || exit 95\nprintf '%s\\n' '{failure}' '{definition}' '{context}' '{candidate}' '{nonce}'\nexit {exit}\n",
            definition = control.definition_digest,
            context = context_id,
            candidate = candidate_id,
            nonce = nonce,
            failure = control.expected_failure_contract,
            exit = EXPECTED_EXIT_CODE,
        );
        let arguments = vec![
            OsString::from("-c"),
            OsString::from(&script),
            OsString::from("hct-claims-named-control"),
            OsString::from(&control.definition_digest),
            OsString::from(context_id),
            OsString::from(candidate_id),
            OsString::from(nonce),
        ];
        let argument_text = arguments
            .iter()
            .map(|value| {
                value
                    .to_str()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| "claims-control-argument-not-utf8".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let argument_digest = digest_bytes(
            &serde_json::to_vec(&argument_text)
                .map_err(|_| "claims-control-argument-encode-failed".to_owned())?,
        );
        let script_digest = digest_value(&script);
        let command_spec_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.registry_digest,
                &control.definition_digest,
                CONTROL_RUNNER,
                &control.runner_digest,
                &argument_digest,
                &script_digest,
                EXPECTED_EXIT_CODE,
            ))
            .map_err(|_| "claims-control-command-spec-encode-failed".to_owned())?,
        );
        let expected_stdout = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            control.expected_failure_contract,
            control.definition_digest,
            context_id,
            candidate_id,
            nonce
        )
        .into_bytes();
        let negative_stimulus_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.claim_id,
                &control.control_id,
                control.control_ordinal,
                &control.definition_digest,
                &command_spec_digest,
                &expected_stdout,
            ))
            .map_err(|_| "claims-control-stimulus-encode-failed".to_owned())?,
        );
        Ok(Self {
            control: control.clone(),
            arguments,
            expected_stdout,
            argument_digest,
            script_digest,
            command_spec_digest,
            negative_stimulus_digest,
            runner_digest: control.runner_digest.clone(),
        })
    }

    fn fixture(&self) -> Result<FixtureSpec, String> {
        FixtureSpec::new(
            self.control.fixture_id.clone(),
            FixtureKind::RewardHacking,
            format!(
                "{}:{}:{}",
                self.control.claim_id, self.control.control_id, self.control.definition_digest
            ),
            BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Process]),
            ExpectedOutcome::causal_failure(self.control.expected_failure_contract.clone(), 0),
            false,
        )
        .map_err(schedule_error)
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecutableIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
}

#[cfg(unix)]
impl ExecutableIdentity {
    fn from(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
        }
    }
}

struct NamedControlAdapter {
    fixture_id: String,
    fixture_digest: String,
    executable: PathBuf,
    executable_digest: String,
    #[cfg(unix)]
    executable_identity: ExecutableIdentity,
    arguments: Vec<OsString>,
    permit_argument_digest: String,
    permit_script_digest: String,
    expected_stdout: Vec<u8>,
    expected_failure_contract: String,
    command_spec_digest: String,
    capture: Mutex<Option<ProcessCapture>>,
}

impl NamedControlAdapter {
    fn issue(fixture: &FixtureSpec, plan: &ControlExecutionPlan) -> Result<Self, String> {
        fixture.validate().map_err(schedule_error)?;
        let executable = PathBuf::from(CONTROL_RUNNER);
        let metadata = protected_executable_metadata(&executable)?;
        let executable_digest = digest_file(&executable)?;
        if executable_digest != plan.runner_digest {
            return Err("claims-control-runner-identity-changed".to_owned());
        }
        Ok(Self {
            fixture_id: fixture.id.clone(),
            fixture_digest: fixture.metadata_digest.clone(),
            executable,
            executable_digest,
            #[cfg(unix)]
            executable_identity: ExecutableIdentity::from(&metadata),
            arguments: plan.arguments.clone(),
            permit_argument_digest: plan.argument_digest.clone(),
            permit_script_digest: plan.script_digest.clone(),
            expected_stdout: plan.expected_stdout.clone(),
            expected_failure_contract: plan.control.expected_failure_contract.clone(),
            command_spec_digest: plan.command_spec_digest.clone(),
            capture: Mutex::new(None),
        })
    }

    fn validate_permit(&self) -> Result<(), FixtureScheduleError> {
        let metadata = protected_executable_metadata(&self.executable)
            .map_err(FixtureScheduleError::Integrity)?;
        #[cfg(unix)]
        if ExecutableIdentity::from(&metadata) != self.executable_identity {
            return Err(FixtureScheduleError::Integrity(
                "claim control executable identity changed".to_owned(),
            ));
        }
        let actual_digest =
            digest_file(&self.executable).map_err(FixtureScheduleError::Integrity)?;
        if actual_digest != self.executable_digest {
            return Err(FixtureScheduleError::Integrity(
                "claim control executable content changed".to_owned(),
            ));
        }
        let argument_text = self
            .arguments
            .iter()
            .map(|value| value.to_str().map(ToOwned::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                FixtureScheduleError::Integrity("claim control argument is not UTF-8".to_owned())
            })?;
        let actual_argument_digest =
            digest_bytes(&serde_json::to_vec(&argument_text).map_err(|_| {
                FixtureScheduleError::Integrity("claim control argument encoding failed".to_owned())
            })?);
        if actual_argument_digest != self.permit_argument_digest
            || self
                .arguments
                .get(1)
                .and_then(|value| value.to_str())
                .map(digest_value)
                .as_deref()
                != Some(self.permit_script_digest.as_str())
        {
            return Err(FixtureScheduleError::Integrity(
                "claim control invocation changed after issue".to_owned(),
            ));
        }
        Ok(())
    }

    fn take_capture(&self) -> Result<Option<ProcessCapture>, String> {
        self.capture
            .lock()
            .map_err(|_| "claims-control-capture-lock-poisoned".to_owned())
            .map(|mut capture| capture.take())
    }
}

impl FixtureExecutor for NamedControlAdapter {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError> {
        if fixture.id != self.fixture_id || fixture.metadata_digest != self.fixture_digest {
            return Err(FixtureScheduleError::Integrity(
                "claim control permit does not bind this fixture".to_owned(),
            ));
        }
        self.validate_permit()?;
        let plan = ConfinementPlan::prepare(&fixture.confinement, lease.root())?;
        let started_at_unix_ms = now_unix_ms().map_err(FixtureScheduleError::Integrity)?;
        let output = plan
            .command(&self.executable, &self.arguments, lease.root(), environment)?
            .output()
            .map_err(FixtureScheduleError::Io)?;
        let ended_at_unix_ms = now_unix_ms()
            .map_err(FixtureScheduleError::Integrity)?
            .max(started_at_unix_ms.saturating_add(1));
        self.validate_permit()?;
        let exit_code = output.status.code().ok_or_else(|| {
            FixtureScheduleError::Integrity("claim control terminated by signal".to_owned())
        })?;
        if exit_code != EXPECTED_EXIT_CODE {
            return Err(FixtureScheduleError::Integrity(format!(
                "claim control exit status was substituted:{exit_code}"
            )));
        }
        if output.stdout != self.expected_stdout {
            return Err(FixtureScheduleError::Integrity(
                "claim control stdout was substituted".to_owned(),
            ));
        }
        if !output.stderr.is_empty() {
            return Err(FixtureScheduleError::Integrity(
                "claim control stderr was not empty".to_owned(),
            ));
        }
        if output.stdout.len() > CONTROL_MAX_OUTPUT_BYTES {
            return Err(FixtureScheduleError::Integrity(
                "claim control output exceeded its permit".to_owned(),
            ));
        }
        let process_digest = digest_bytes(
            &serde_json::to_vec(&(
                &self.fixture_digest,
                &self.command_spec_digest,
                &self.executable_digest,
                started_at_unix_ms,
                ended_at_unix_ms,
                exit_code,
                &output.stdout,
                &output.stderr,
            ))
            .map_err(|_| {
                FixtureScheduleError::Integrity("claim control capture encoding failed".to_owned())
            })?,
        );
        let mut capture = self.capture.lock().map_err(|_| {
            FixtureScheduleError::Integrity("claim control capture lock poisoned".to_owned())
        })?;
        if capture.is_some() {
            return Err(FixtureScheduleError::Integrity(
                "claim control permit was replayed".to_owned(),
            ));
        }
        *capture = Some(ProcessCapture {
            started_at_unix_ms,
            ended_at_unix_ms,
            exit_code,
            process_digest,
        });
        Ok(ObservedOutcome::failure(&self.expected_failure_contract, 0))
    }
}

struct ProcessCapture {
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
    exit_code: i32,
    process_digest: String,
}

#[cfg(test)]
fn run_substituted_fixture(
    batch_root: &Path,
    fixture: FixtureSpec,
    adapter: &NamedControlAdapter,
) -> Result<ExecutionObservation, String> {
    let mut scheduler = FixtureScheduler::new(batch_root);
    let lease = scheduler
        .schedule([fixture])
        .map_err(schedule_error)?
        .pop()
        .ok_or_else(|| "claims-control-fixture-not-scheduled".to_owned())?;
    scheduler.execute(&lease, adapter).map_err(schedule_error)?;
    Err("claims-control-substituted-fixture-accepted".to_owned())
}

fn candidate_id(context: &LiveContext) -> Result<String, String> {
    let bytes = serde_json::to_vec(context.candidate())
        .map_err(|_| "claims-control-candidate-encode-failed".to_owned())?;
    Ok(digest_bytes(&bytes))
}

fn validate_scratch_root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("claims-control-scratch-root-invalid".to_owned());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "claims-control-scratch-root-unavailable".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("claims-control-scratch-root-unsafe".to_owned());
    }
    #[cfg(unix)]
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
        return Err("claims-control-scratch-root-unsafe".to_owned());
    }
    path.canonicalize()
        .map_err(|_| "claims-control-scratch-root-unavailable".to_owned())
}

fn protected_executable_metadata(path: &Path) -> Result<fs::Metadata, String> {
    if !path.is_absolute() {
        return Err("claims-control-runner-path-invalid".to_owned());
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "claims-control-runner-unavailable".to_owned())?;
    #[cfg(unix)]
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return Err("claims-control-runner-identity-unsafe".to_owned());
    }
    #[cfg(not(unix))]
    if !metadata.is_file() {
        return Err("claims-control-runner-identity-unsafe".to_owned());
    }
    Ok(metadata)
}

fn protected_executable_digest(path: &Path) -> Result<String, String> {
    protected_executable_metadata(path)?;
    digest_file(path)
}

fn digest_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|_| "claims-control-runner-read-failed".to_owned())?;
    Ok(digest_bytes(&bytes))
}

fn now_unix_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "claims-control-clock-before-epoch".to_owned())
        .map(|duration| duration.as_millis() as u64)
}

fn short_digest(value: &str) -> &str {
    value
        .strip_prefix("sha256:")
        .and_then(|hex| hex.get(..16))
        .unwrap_or("invalid-digest")
}

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn digest_value(value: &str) -> String {
    digest_bytes(value.as_bytes())
}

fn schedule_error(error: FixtureScheduleError) -> String {
    format!("claims-control-fixture-scheduler:{error}")
}
