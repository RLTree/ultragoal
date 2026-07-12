use super::definition::{ADOPTED_CLAIM_REGISTRY_SHA256, ClaimDefinition, ClaimDefinitions};
#[cfg(test)]
use super::evidence::{Actor, ActorRole};
pub use super::false_pass_receipt::{SemanticControlModel, SemanticControlObservation};
use crate::context::LiveContext;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub(super) const MODEL_METHOD: &str = "claim-control-semantic-model-v1";
pub(super) const MODEL_OBSERVATION_METHOD: &str = "claim-control-semantic-model-observation-v1";
const CONTROL_CATALOG_VERSION: &str = "adopted-claim-negative-controls-v1";
const MODEL_IMPLEMENTATION_VERSION: &str = "claim-control-decision-test-model-v1";
const MODEL_MAX_AGE_MS: u64 = 600_000;
pub(crate) const PRIVATE_TRANSPORT_UNAVAILABLE: &str =
    "claims-control-transport-unavailable:confinement-cleanup-capability-intersection-empty";
#[cfg(test)]
static MODEL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Private capability for producing sealed semantic models in claim-decision
/// tests. It cannot issue or represent executed causal proof.
pub(super) struct ModelAuthority {
    _private: (),
}

impl ModelAuthority {
    #[cfg(test)]
    fn issue_for_decision_tests() -> Self {
        Self { _private: () }
    }
}

/// Candidate-bound claim-control authority.
///
/// No adopted product executor currently satisfies the required confinement
/// and cleanup capability intersection. The production and private transports
/// therefore refuse before any scheduling, process, receipt, or filesystem
/// effect. A separate test-only semantic model exercises decision logic and is
/// rejected by the default/product ledger policy.
pub struct LocalNegativeControlAuthority<'a> {
    definitions: &'a ClaimDefinitions,
    context: LiveContext,
    context_id: String,
    candidate_id: String,
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
        validate_scratch_root(scratch_root.as_ref())?;
        validate_registry_control_index(definitions)?;
        Ok(Self {
            definitions,
            context: context.clone(),
            context_id,
            candidate_id,
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
    ) -> Result<BTreeMap<String, SemanticControlObservation>, String> {
        let _ = claim_id;
        #[cfg(not(test))]
        return product_executor_catalog_preflight();
        #[cfg(test)]
        Err(PRIVATE_TRANSPORT_UNAVAILABLE.to_owned())
    }

    /// Creates sealed semantic control models for decision-engine tests only.
    /// Models contain no process capture, exit status, executor identity,
    /// execution timestamp, output artifact, or causal outcome.
    #[cfg(test)]
    pub(crate) fn model_claim_for_decision_tests(
        &self,
        claim_id: &str,
    ) -> Result<BTreeMap<String, SemanticControlObservation>, String> {
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
            let observation = self.model_control(&control)?;
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

    #[cfg(test)]
    fn model_control(
        &self,
        control: &NamedControlDefinition,
    ) -> Result<SemanticControlObservation, String> {
        let sequence = MODEL_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
        let modeled_at_unix_ms = now_unix_ms()?;
        let nonce = digest_value(&format!(
            "claim-control-semantic-model-nonce-v1:{}:{}:{}:{}",
            control.definition_digest, self.context_id, self.candidate_id, sequence
        ));
        let plan =
            SemanticControlPlan::issue(control, &self.context_id, &self.candidate_id, &nonce)?;
        let authority = ModelAuthority::issue_for_decision_tests();
        let model_id = digest_value(&format!(
            "claim-control-semantic-model-id-v1:{}:{}:{}",
            plan.model_spec_digest, nonce, plan.modeled_result_digest
        ));
        let model = SemanticControlModel::from_authority(
            &authority,
            model_id,
            self.definitions.registry_digest().to_owned(),
            control.claim_id.clone(),
            control.control_id.clone(),
            control.definition_digest.clone(),
            plan.model_spec_digest,
            nonce,
            plan.negative_stimulus_digest,
            control.expected_failure_contract.clone(),
            Actor {
                actor_id: expected_modeler_id(control),
                roles: BTreeSet::from([ActorRole::SemanticModeler]),
            },
            format!("{MODEL_METHOD}:{sequence}"),
            plan.model_implementation_digest,
            modeled_at_unix_ms,
            self.context_id.clone(),
            self.candidate_id.clone(),
            MODEL_MAX_AGE_MS,
            control.truth_surface.clone(),
            control.declared_ceiling.clone(),
            plan.modeled_result_digest,
        )?;
        SemanticControlObservation::from_authority(
            &authority,
            model,
            Actor {
                actor_id: model_observer_id(control),
                roles: BTreeSet::from([
                    ActorRole::IndependentObserver,
                    ActorRole::SemanticModelObserver,
                ]),
            },
            format!("{MODEL_OBSERVATION_METHOD}:{sequence}"),
            now_unix_ms()?.max(modeled_at_unix_ms),
        )
    }

    #[cfg(test)]
    pub(crate) fn execute_substituted_for_test(
        &self,
        claim_id: &str,
        control_id: &str,
        substitution: TestSubstitution,
    ) -> Result<SemanticControlObservation, String> {
        let _ = (claim_id, control_id, substitution);
        Err(PRIVATE_TRANSPORT_UNAVAILABLE.to_owned())
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestSubstitution {
    GenericTool,
    Arguments,
    FixtureSpec,
}

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
    pub(super) definition_digest: String,
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
        let base = serde_json::to_vec(&(
            CONTROL_CATALOG_VERSION,
            registry_digest,
            &definition.claim_id,
            control_id,
            control_ordinal,
            &definition.truth_surface,
            &definition.allowed_ceiling_on_pass,
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
            definition_digest,
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

pub(super) fn validate_named_control_model(
    registry_digest: &str,
    definition: &ClaimDefinition,
    model: &SemanticControlModel,
) -> Result<(), String> {
    let expected = expected_control_definition(registry_digest, definition, model.control_id())?;
    let plan = SemanticControlPlan::issue(
        &expected,
        model.live_context_id(),
        model.candidate_id(),
        model.authority_nonce(),
    )?;
    let expected_model_id = digest_value(&format!(
        "claim-control-semantic-model-id-v1:{}:{}:{}",
        plan.model_spec_digest,
        model.authority_nonce(),
        plan.modeled_result_digest
    ));
    if model.registry_digest() != registry_digest
        || model.claim_id() != definition.claim_id
        || model.control_id() != expected.control_id
        || model.control_definition_digest() != expected.definition_digest
        || model.model_spec_digest() != plan.model_spec_digest
        || model.negative_stimulus_digest() != plan.negative_stimulus_digest
        || model.expected_failure_contract() != expected.expected_failure_contract
        || model.model_implementation_digest() != plan.model_implementation_digest
        || model.modeler().actor_id != expected_modeler_id(&expected)
        || model.truth_surface() != expected.truth_surface
        || model.declared_ceiling() != expected.declared_ceiling
        || model.max_age_ms() != MODEL_MAX_AGE_MS
        || model.modeled_result_digest() != plan.modeled_result_digest
        || model.model_id() != expected_model_id
        || method_sequence(model.model_method(), MODEL_METHOD).is_none()
    {
        return Err("claims-control-semantic-model-definition-mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn expected_model_observer_id(
    registry_digest: &str,
    definition: &ClaimDefinition,
    control_id: &str,
) -> Result<String, String> {
    let expected = expected_control_definition(registry_digest, definition, control_id)?;
    Ok(model_observer_id(&expected))
}

fn expected_modeler_id(control: &NamedControlDefinition) -> String {
    format!(
        "claim-control-modeler:{}",
        short_digest(&control.definition_digest)
    )
}

fn model_observer_id(control: &NamedControlDefinition) -> String {
    format!(
        "claim-control-model-observer:{}",
        short_digest(&control.definition_digest)
    )
}

pub(super) fn method_sequence(method: &str, prefix: &str) -> Option<u64> {
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
        for control_id in &definition.false_pass_controls {
            if !control_bindings.insert((claim_id.clone(), control_id.clone())) {
                return Err("claims-control-catalog-conflict".to_owned());
            }
        }
    }
    let expected = definitions
        .order()
        .iter()
        .map(|claim_id| {
            definitions
                .definition(claim_id)
                .map(|definition| definition.false_pass_controls.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    if control_bindings.len() != expected {
        return Err("claims-control-catalog-incomplete".to_owned());
    }
    Ok(())
}

struct SemanticControlPlan {
    model_spec_digest: String,
    negative_stimulus_digest: String,
    model_implementation_digest: String,
    modeled_result_digest: String,
}

impl SemanticControlPlan {
    fn issue(
        control: &NamedControlDefinition,
        context_id: &str,
        candidate_id: &str,
        nonce: &str,
    ) -> Result<Self, String> {
        if !digest(context_id) || !digest(candidate_id) || !digest(nonce) {
            return Err("claims-control-model-binding-not-digest".to_owned());
        }
        let model_implementation_digest = digest_value(MODEL_IMPLEMENTATION_VERSION);
        let model_spec_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.registry_digest,
                &control.claim_id,
                &control.control_id,
                control.control_ordinal,
                &control.definition_digest,
                &control.expected_failure_contract,
                &model_implementation_digest,
            ))
            .map_err(|_| "claims-control-model-spec-encode-failed".to_owned())?,
        );
        let negative_stimulus_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.claim_id,
                &control.control_id,
                control.control_ordinal,
                &control.definition_digest,
                &model_spec_digest,
                context_id,
                candidate_id,
                nonce,
            ))
            .map_err(|_| "claims-control-model-stimulus-encode-failed".to_owned())?,
        );
        let modeled_result_digest = digest_bytes(
            &serde_json::to_vec(&(
                "semantic-model-not-executed",
                &control.expected_failure_contract,
                &negative_stimulus_digest,
                &model_spec_digest,
            ))
            .map_err(|_| "claims-control-modeled-result-encode-failed".to_owned())?,
        );
        Ok(Self {
            model_spec_digest,
            negative_stimulus_digest,
            model_implementation_digest,
            modeled_result_digest,
        })
    }
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
