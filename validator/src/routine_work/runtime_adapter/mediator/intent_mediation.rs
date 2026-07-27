use super::super::RUST_SOURCE_SYNTAX_BEHAVIOR;
use super::outcome::ExecutedIntentProjection;
use super::*;

#[must_use = "the exact prepared intent must receive one process observation"]
pub(crate) struct IntentExecutionRequest {
    token: RoutineMediatedIntent,
    snapshot_id: String,
    dependencies: BTreeMap<String, String>,
    root: RootAnchor,
    program: PinnedExecutable,
    outputs: OutputConfinement,
    reads: ReadConfinement,
    environment: BTreeMap<String, String>,
    framed_input: Vec<u8>,
    framed_input_sha256: String,
    cancellation: RoutineCancellation,
}

pub(super) struct PreparedIntentExecution {
    snapshot_id: String,
    dependencies: BTreeMap<String, String>,
    root: RootAnchor,
    program: PinnedExecutable,
    outputs: OutputConfinement,
    reads: ReadConfinement,
    environment: BTreeMap<String, String>,
    framed_input: Vec<u8>,
    framed_input_sha256: String,
}

pub(super) fn prepare_intent(
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    snapshot_id: &str,
    dependencies: BTreeMap<String, String>,
) -> Result<PreparedIntentExecution, RoutineError> {
    token.require_current()?;
    validate_intent(context, plan, token.intent())?;
    let root = RootAnchor::open(context.worktree_root())?;
    let program = PinnedExecutable::open_bound(
        token.intent().program_path_hex(),
        token.intent().program_sha256(),
        token.intent().program_byte_length(),
        token.intent().program_unix_mode(),
    )?;
    let outputs = OutputConfinement::prepare(
        &root,
        token.intent().declared_output_scopes(),
        token.intent().output_budget_bytes(),
    )?;
    let reads = ReadConfinement::open_bound(&root, token.intent().read_sources())?;
    let environment = execution_environment(token)?;
    if token.intent().behavior_id() != RUST_SOURCE_SYNTAX_BEHAVIOR {
        return Err(mediator_error("mediator-behavior-unsupported"));
    }
    let framed_input = reads.rust_source_syntax_frame(&root)?;
    Ok(PreparedIntentExecution {
        snapshot_id: snapshot_id.to_owned(),
        dependencies,
        root,
        program,
        outputs,
        reads,
        environment,
        framed_input_sha256: sha256(&framed_input),
        framed_input,
    })
}

pub(super) fn bind_intent_request(
    token: RoutineMediatedIntent,
    prepared: PreparedIntentExecution,
    cancellation: RoutineCancellation,
) -> IntentExecutionRequest {
    IntentExecutionRequest {
        token,
        snapshot_id: prepared.snapshot_id,
        dependencies: prepared.dependencies,
        root: prepared.root,
        program: prepared.program,
        outputs: prepared.outputs,
        reads: prepared.reads,
        environment: prepared.environment,
        framed_input: prepared.framed_input,
        framed_input_sha256: prepared.framed_input_sha256,
        cancellation,
    }
}

impl IntentExecutionRequest {
    pub(crate) fn program(&self) -> &PinnedExecutable {
        &self.program
    }

    pub(crate) fn root(&self) -> &RootAnchor {
        &self.root
    }

    pub(crate) fn outputs(&self) -> &OutputConfinement {
        &self.outputs
    }

    pub(crate) fn reads(&self) -> &ReadConfinement {
        &self.reads
    }

    pub(crate) fn argv(&self) -> &[String] {
        self.token.intent().argv()
    }

    pub(crate) fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    pub(crate) fn framed_input(&self) -> &[u8] {
        &self.framed_input
    }

    pub(crate) fn timeout(&self) -> Duration {
        Duration::from_millis(self.token.intent().timeout_ms())
    }

    pub(crate) fn output_budget(&self) -> u64 {
        self.token.intent().output_budget_bytes()
    }

    pub(crate) fn cancellation(&self) -> &RoutineCancellation {
        &self.cancellation
    }

    pub(crate) fn node_id(&self) -> &str {
        self.token.intent().node_id()
    }

    pub(crate) fn intent_id(&self) -> &str {
        self.token.intent().intent_id()
    }

    pub(crate) fn plan_order(&self) -> usize {
        self.token.intent().plan_order()
    }

    pub(super) fn observe(
        self,
        context: &LiveContext,
        plan: &RoutinePlan,
        observation: ProcessObservation,
    ) -> Result<IntentResult, RoutineError> {
        self.reads.validate(&self.root)?;
        let disposition = match observation.termination {
            ProcessTermination::Exited(0) => None,
            ProcessTermination::Cancelled => {
                Some((RoutineNodeDisposition::Cancelled, "MEDIATOR-CANCELLED"))
            }
            ProcessTermination::TimedOut => {
                Some((RoutineNodeDisposition::Failed, "MEDIATOR-TIMEOUT"))
            }
            ProcessTermination::OutputLimit => {
                Some((RoutineNodeDisposition::Failed, "MEDIATOR-OUTPUT-LIMIT"))
            }
            ProcessTermination::DescendantSurvived => Some((
                RoutineNodeDisposition::Failed,
                "MEDIATOR-DESCENDANT-SURVIVED",
            )),
            ProcessTermination::CleanupFailed => {
                return Err(mediator_error("mediator-process-cleanup-failed"));
            }
            ProcessTermination::Exited(_) | ProcessTermination::Signaled(_) => {
                Some((RoutineNodeDisposition::Failed, "MEDIATOR-CHECK-FAILED"))
            }
        };
        if let Some((disposition, failure_code)) = disposition {
            let result = IntentResult::Incomplete {
                disposition,
                failure_code,
            };
            self.token.advance()?;
            return Ok(result);
        }
        if validate_rust_source_observation(Some(&self.framed_input), &observation).is_err() {
            let result = IntentResult::Incomplete {
                disposition: RoutineNodeDisposition::Failed,
                failure_code: "MEDIATOR-BEHAVIOR-OBSERVATION-INVALID",
            };
            self.token.advance()?;
            return Ok(result);
        }
        let output_files = self.outputs.capture_owned_delta()?;
        let artifact_bytes = output_files
            .values()
            .try_fold(0_u64, |total, file| total.checked_add(file.byte_length));
        if artifact_bytes
            .and_then(|total| total.checked_add(observation.output_byte_length))
            .is_none_or(|total| total > self.output_budget())
        {
            let result = IntentResult::Incomplete {
                disposition: RoutineNodeDisposition::Failed,
                failure_code: "MEDIATOR-OUTPUT-LIMIT",
            };
            self.token.advance()?;
            return Ok(result);
        }
        self.outputs.validate()?;
        self.reads.validate(&self.root)?;
        self.root.validate()?;
        self.program.validate()?;
        context
            .revalidate()
            .map_err(|_| concurrent("mediator-context-mutated-by-process"))?;
        validate_snapshot(context, &self.snapshot_id)?;
        let result = project_executed_intent(
            context,
            plan,
            &self.token,
            ExecutedIntentProjection {
                snapshot_id: &self.snapshot_id,
                dependencies: &self.dependencies,
                framed_input_sha256: &self.framed_input_sha256,
                observation: &observation,
                output_files,
            },
        )?;
        self.token.advance()?;
        Ok(IntentResult::Executed(result))
    }
}
