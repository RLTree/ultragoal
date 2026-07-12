use super::artifact;
use super::artifact_model::CapturedArtifact;
use super::environment::{self, ArgumentRecord, EnvironmentRecord, InvocationSensitivity};
use super::filesystem::{RootAnchor, validate_context_roots};
use super::output::CapturedOutput;
use super::process::{self, Termination};
use super::program::PinnedProgram;
use super::sandbox::{EnforcementRecord, SandboxPlan};
use super::spec::CommandSpec;
use super::tree::TreeSnapshot;
use super::util::{bytes_hex, context_candidate_id, os_bytes};
use crate::context::{BuildRequest, EffectClass, LiveContext};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct CandidateBinding {
    candidate_id: String,
    head_commit: Option<String>,
    head_tree: Option<String>,
    status_sha256: String,
    worktree_diff_sha256: String,
    staged_diff_sha256: String,
    untracked_content_sha256: String,
    dirty: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum InvocationMetadataDisposition {
    Public,
    WithheldSecretBearingInvocation,
}

impl InvocationMetadataDisposition {
    #[cfg(test)]
    const fn is_withheld(self) -> bool {
        matches!(self, Self::WithheldSecretBearingInvocation)
    }
}

impl From<InvocationSensitivity> for InvocationMetadataDisposition {
    fn from(sensitivity: InvocationSensitivity) -> Self {
        match sensitivity {
            InvocationSensitivity::Public => Self::Public,
            InvocationSensitivity::SecretBearing => Self::WithheldSecretBearingInvocation,
        }
    }
}

/// A typed observation of process behavior. No field interprets claim success.
#[derive(Debug, Serialize)]
pub struct CapturedRun {
    schema_version: &'static str,
    tool_id: &'static str,
    command_id: String,
    authority_context_id: String,
    observed_context_id: String,
    candidate_before: CandidateBinding,
    candidate_after: CandidateBinding,
    effect: EffectClass,
    invocation_metadata_disposition: InvocationMetadataDisposition,
    program_path_hex: String,
    program_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd_path_hex: Option<String>,
    arguments: Vec<ArgumentRecord>,
    environment: Vec<EnvironmentRecord>,
    enforcement: EnforcementRecord,
    termination: Termination,
    monotonic_duration_ns: u64,
    stdout: CapturedOutput,
    stderr: CapturedOutput,
    artifacts: Vec<CapturedArtifact>,
}

impl CapturedRun {
    pub fn context_id(&self) -> &str {
        &self.authority_context_id
    }

    pub fn observed_context_id(&self) -> &str {
        &self.observed_context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_after.candidate_id
    }

    pub fn authority_candidate_id(&self) -> &str {
        &self.candidate_before.candidate_id
    }

    pub fn interrupted(&self) -> bool {
        matches!(self.termination, Termination::Interrupted)
    }

    pub fn monotonic_duration_ns(&self) -> u64 {
        self.monotonic_duration_ns
    }

    pub fn stdout_bytes(&self) -> &[u8] {
        self.stdout.retained()
    }

    pub fn stderr_bytes(&self) -> &[u8] {
        self.stderr.retained()
    }

    pub fn artifacts(&self) -> &[CapturedArtifact] {
        &self.artifacts
    }

    pub fn to_canonical_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "captured run serialization failed".to_owned())
    }

    #[cfg(test)]
    pub(super) fn install_output_limit_for_test(
        &mut self,
        stdout: CapturedOutput,
        stderr: CapturedOutput,
    ) {
        self.termination = if self.invocation_metadata_disposition.is_withheld() {
            Termination::WithheldSecretBearingInvocation
        } else {
            Termination::OutputLimit
        };
        if self.invocation_metadata_disposition.is_withheld() {
            self.monotonic_duration_ns = 0;
        }
        self.stdout = stdout;
        self.stderr = stderr;
    }
}

fn binding(context: &LiveContext, candidate_id: String) -> CandidateBinding {
    let candidate = context.candidate();
    CandidateBinding {
        candidate_id,
        head_commit: candidate.head_commit.clone(),
        head_tree: candidate.head_tree.clone(),
        status_sha256: candidate.status_sha256.clone(),
        worktree_diff_sha256: candidate.worktree_diff_sha256.clone(),
        staged_diff_sha256: candidate.staged_diff_sha256.clone(),
        untracked_content_sha256: candidate.untracked_content_sha256.clone(),
        dirty: candidate.dirty,
    }
}

fn refreshed_context(source: &LiveContext) -> Result<LiveContext, String> {
    let mut request = BuildRequest::new(source.worktree_root())
        .expect_repository_root(PathBuf::from(&source.roots().repository_root))
        .expect_worktree_root(PathBuf::from(&source.roots().worktree_root));
    for (name, value) in &source.configuration().public_values {
        request = request.bind_non_secret_configuration(name, value);
    }
    for secret in &source.configuration().secret_sources {
        request = request.bind_secret_source(&secret.name, &secret.public_version);
    }
    for input in source.selected_inputs() {
        request = request.select_input(PathBuf::from(&input.relative_path));
    }
    for tool in &source.capabilities().tools {
        request = request.probe_tool(&tool.name);
    }
    LiveContext::build(request)
        .map_err(|_| "post-execution live context could not be built".to_owned())
}

pub(super) fn capture(spec: &CommandSpec, context: &LiveContext) -> Result<CapturedRun, String> {
    spec.validate()?;
    validate_context_roots(context)?;
    let catalog_binding = spec.require_catalog_binding()?;
    context
        .revalidate()
        .map_err(|_| "live context failed execution-time revalidation".to_owned())?;
    context
        .effect()
        .authorize(spec.effect)
        .map_err(|_| "command effect is not structurally authorized".to_owned())?;
    let authority_candidate_id = context_candidate_id(context)?;
    let environment = environment::prepare(spec, context)?;
    let sandbox = SandboxPlan::prepare(context, spec.effect)?;
    let root = RootAnchor::new(context)?;
    let program = PinnedProgram::open(context, &catalog_binding.capability)?;
    let cwd = root.open_directory(&spec.cwd)?;
    context
        .revalidate()
        .map_err(|_| "live context changed before process start".to_owned())?;
    let read_witness = TreeSnapshot::stable(&root)?;
    context
        .revalidate()
        .map_err(|_| "live context changed while binding the process".to_owned())?;
    program.validate()?;
    cwd.validate(&root)?;
    root.validate()?;
    context
        .revalidate()
        .map_err(|_| "live context changed before enforced process launch".to_owned())?;
    let result = process::execute(
        &program,
        &cwd,
        &environment,
        spec.timeout,
        spec.output_limit,
        spec.observed_output_limit,
        spec.interrupt.as_ref(),
        &sandbox,
    )?;
    program.validate()?;
    cwd.validate(&root)?;
    root.validate()?;
    read_witness.validate(&root)?;
    context
        .revalidate()
        .map_err(|_| "read-class subprocess changed its live context".to_owned())?;
    let observed_context = refreshed_context(context)?;
    let observed_candidate_id = context_candidate_id(&observed_context)?;
    let artifacts = artifact::capture(
        &root,
        &spec.artifacts,
        observed_context.context_id(),
        &observed_candidate_id,
        environment.sensitivity,
    )?;
    root.validate()?;
    observed_context
        .revalidate()
        .map_err(|_| "artifact capture observed candidate drift".to_owned())?;
    Ok(CapturedRun {
        schema_version: "CapturedRun-v1",
        tool_id: "HCT-CAPTURE",
        command_id: catalog_binding.command_id.clone(),
        authority_context_id: context.context_id().to_owned(),
        observed_context_id: observed_context.context_id().to_owned(),
        candidate_before: binding(context, authority_candidate_id),
        candidate_after: binding(&observed_context, observed_candidate_id),
        effect: spec.effect,
        invocation_metadata_disposition: environment.sensitivity.into(),
        program_path_hex: bytes_hex(os_bytes(program.path().as_os_str())),
        program_sha256: program.sha256().to_owned(),
        cwd_path_hex: (!environment.sensitivity.is_secret_bearing())
            .then(|| bytes_hex(os_bytes(spec.cwd.as_os_str()))),
        arguments: environment.argument_records,
        environment: environment.records,
        enforcement: sandbox.record(true, environment.sensitivity),
        termination: result.termination,
        monotonic_duration_ns: result.duration_ns,
        stdout: result.stdout,
        stderr: result.stderr,
        artifacts,
    })
}
