use super::super::{ProductError, ProductWorkspace};
use super::{OrchestrationStateView, diagnose, next};
use crate::orchestration::OrchestrationError;
use serde::{Deserialize, Serialize};

const MAX_COMMAND_OUTPUT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projection", content = "finding_id", rename_all = "snake_case")]
pub enum CommandProjection {
    Inspect,
    Next,
    Diagnose(Option<String>),
}

/// Produces bounded deterministic bytes for the existing inspect, next, and
/// diagnose grammar. Root-owned routing remains responsible for integration.
pub fn project(
    view: &OrchestrationStateView,
    workspace: &ProductWorkspace,
    projection: &CommandProjection,
) -> Result<Option<Vec<u8>>, ProductError> {
    view.revalidate(workspace)?;
    let bytes = match projection {
        CommandProjection::Inspect => serde_json::to_vec(view),
        CommandProjection::Next => serde_json::to_vec(&next(view)),
        CommandProjection::Diagnose(finding_id) => {
            let Some(diagnosis) = diagnose(view, finding_id.as_deref())? else {
                return Ok(None);
            };
            serde_json::to_vec(&diagnosis)
        }
    }
    .map_err(|_| ProductError::StaleCandidate)?;
    if bytes.len() > MAX_COMMAND_OUTPUT {
        return Err(ProductError::Kernel(OrchestrationError::ResourceLimit));
    }
    view.revalidate(workspace)?;
    Ok(Some(bytes))
}
