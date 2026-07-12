use super::types::{CeilingReduction, Repair, Scope};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeField {
    ModelFamily,
    Model,
    Mode,
    Reasoning,
    RuntimeConfiguration,
}

impl RuntimeField {
    pub(crate) const ALL: [Self; 5] = [
        Self::ModelFamily,
        Self::Model,
        Self::Mode,
        Self::Reasoning,
        Self::RuntimeConfiguration,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ModelFamily => "model-family",
            Self::Model => "model",
            Self::Mode => "mode",
            Self::Reasoning => "reasoning",
            Self::RuntimeConfiguration => "runtime-configuration",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeSource {
    CodexExposed,
    RuntimeExposed,
}

/// Context-bound runtime metadata. Its fields are intentionally not publicly constructible.
///
/// ```compile_fail
/// use ultragoal::state::{RuntimeSource, RuntimeValue};
/// let _forged = RuntimeValue {
///     value: "prompt-request".to_owned(),
///     source: RuntimeSource::CodexExposed,
///     exposed_source: "prompt".to_owned(),
///     context_id: "sha256:forged".to_owned(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeValue {
    value: String,
    source: RuntimeSource,
    exposed_source: String,
    context_id: String,
}

impl RuntimeValue {
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn source(&self) -> RuntimeSource {
        self.source
    }
    pub fn exposed_source(&self) -> &str {
        &self.exposed_source
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    #[cfg(test)]
    pub(crate) fn test_exposed(
        value: &str,
        source: RuntimeSource,
        exposed_source: &str,
        context_id: &str,
    ) -> Self {
        Self {
            value: value.to_owned(),
            source,
            exposed_source: exposed_source.to_owned(),
            context_id: context_id.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RuntimeMetadata {
    pub model_family: Option<RuntimeValue>,
    pub model: Option<RuntimeValue>,
    pub mode: Option<RuntimeValue>,
    pub reasoning: Option<RuntimeValue>,
    pub runtime_configuration: Option<RuntimeValue>,
}

impl RuntimeMetadata {
    pub(crate) fn value(&self, field: RuntimeField) -> Option<&RuntimeValue> {
        match field {
            RuntimeField::ModelFamily => self.model_family.as_ref(),
            RuntimeField::Model => self.model.as_ref(),
            RuntimeField::Mode => self.mode.as_ref(),
            RuntimeField::Reasoning => self.reasoning.as_ref(),
            RuntimeField::RuntimeConfiguration => self.runtime_configuration.as_ref(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeRequirement {
    pub field: RuntimeField,
    pub scope: Scope,
    pub repair: Repair,
    pub ceiling_reductions: Vec<CeilingReduction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostGoalStatus {
    Unavailable,
    Active,
    Paused,
    Complete,
}

/// A host goal observation cannot be constructed by an external state-catalog caller.
///
/// ```compile_fail
/// use ultragoal::state::{HostGoalObservation, HostGoalStatus};
/// let _forged = HostGoalObservation {
///     status: HostGoalStatus::Complete,
///     exposed_source: Some("prompt".to_owned()),
///     context_id: Some("sha256:forged".to_owned()),
///     authoritative_for_product_claims: true,
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostGoalObservation {
    status: HostGoalStatus,
    exposed_source: Option<String>,
    context_id: Option<String>,
    authoritative_for_product_claims: bool,
}

impl HostGoalObservation {
    pub fn status(&self) -> HostGoalStatus {
        self.status
    }
    pub fn exposed_source(&self) -> Option<&str> {
        self.exposed_source.as_deref()
    }
    pub fn context_id(&self) -> Option<&str> {
        self.context_id.as_deref()
    }
    pub fn authoritative_for_product_claims(&self) -> bool {
        false
    }

    #[cfg(test)]
    pub(crate) fn test_exposed(status: HostGoalStatus, source: &str, context_id: &str) -> Self {
        Self {
            status,
            exposed_source: Some(source.to_owned()),
            context_id: Some(context_id.to_owned()),
            authoritative_for_product_claims: false,
        }
    }
}

impl Default for HostGoalObservation {
    fn default() -> Self {
        Self {
            status: HostGoalStatus::Unavailable,
            exposed_source: None,
            context_id: None,
            authoritative_for_product_claims: false,
        }
    }
}
