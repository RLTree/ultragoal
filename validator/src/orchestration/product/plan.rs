use super::context::{ReadOnlySink, open_engine};
use super::snapshot::snapshot;
use super::{ProductContext, ProductError, ProductWorkspace};
use crate::orchestration::{Blocker, JournalHead, Plan, RecoveryPlan};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub blockers: Vec<Blocker>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPlan {
    pub schema_version: String,
    pub snapshot_id: String,
    pub schedule: Plan,
    pub recovery: RecoveryPlan,
}

impl ProductPlan {
    pub fn plan_id(&self) -> Result<String, ProductError> {
        let bytes = serde_json::to_vec(self).map_err(|_| ProductError::StaleCandidate)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

/// Recursively read-only dependency and recovery planning.
pub fn plan(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    request: &PlanRequest,
) -> Result<ProductPlan, ProductError> {
    let engine = open_engine(context, workspace, &request.expected_head, ReadOnlySink)?;
    let current = snapshot(&engine, request.tick, &request.live_workers)?;
    let result = ProductPlan {
        schema_version: "OrchestrationProductPlan-v1".to_owned(),
        snapshot_id: current.snapshot_id()?,
        schedule: current.plan,
        recovery: engine
            .recovery_plan(&request.blockers)
            .map_err(ProductError::from)?,
    };
    workspace.verify()?;
    Ok(result)
}
