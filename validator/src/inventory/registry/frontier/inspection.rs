use super::validated_snapshot;
use crate::context::LiveContext;
use crate::inventory::digest::sha256_hex;
use crate::inventory::types::InventoryError;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct Projection {
    pub(crate) schema_version: &'static str,
    pub(crate) context_id: String,
    pub(crate) candidate: Candidate,
    pub(crate) registry_sha256: String,
    pub(crate) frontier: String,
    pub(crate) lanes: Vec<Lane>,
    pub(crate) lifecycle_counts: BTreeMap<String, usize>,
    pub(crate) claim_ceiling: &'static str,
}

#[derive(Serialize)]
pub(crate) struct Candidate {
    head_commit: Option<String>,
    head_tree: Option<String>,
    dirty: bool,
}

#[derive(Serialize)]
pub(crate) struct Lane {
    id: String,
    lifecycle: String,
}

pub(crate) fn project(context: &LiveContext) -> Result<Projection, InventoryError> {
    context
        .revalidate()
        .map_err(|error| unavailable(error.to_string()))?;
    let reads = context
        .begin_read_session()
        .map_err(|error| unavailable(error.to_string()))?;
    let root = reads.root();
    let registry_path = root.join("LANE_REGISTRY.json");
    let registry_bytes =
        crate::inventory::fs::read_bounded(&reads, &registry_path, 2 * 1024 * 1024)?;
    let snapshot = validated_snapshot::load(&reads, root)?;
    let lanes = lanes(&snapshot.registry)?;
    reads
        .revalidate()
        .map_err(|error| unavailable(error.to_string()))?;
    context
        .revalidate()
        .map_err(|error| unavailable(error.to_string()))?;
    let lifecycle_counts = lifecycle_counts(&lanes);
    Ok(Projection {
        schema_version: "HarnessOrchestrationInspection-v1",
        context_id: context.context_id().to_owned(),
        candidate: Candidate {
            head_commit: context.candidate().head_commit.clone(),
            head_tree: context.candidate().head_tree.clone(),
            dirty: context.candidate().dirty,
        },
        registry_sha256: format!("sha256:{}", sha256_hex(&registry_bytes)),
        frontier: text(
            snapshot.registry.pointer("/pre_adoption_source/frontier"),
            "scheduler frontier is missing",
        )?
        .to_owned(),
        lanes,
        lifecycle_counts,
        claim_ceiling: "orchestration inspection is read-only; product, runtime, readiness, release, and completion claims remain withheld",
    })
}

fn lifecycle_counts(lanes: &[Lane]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for lane in lanes {
        *counts.entry(lane.lifecycle.clone()).or_insert(0) += 1;
    }
    counts
}

fn lanes(registry: &Value) -> Result<Vec<Lane>, InventoryError> {
    registry
        .get("lanes")
        .and_then(Value::as_array)
        .ok_or_else(|| unavailable("scheduler lanes are missing".to_owned()))?
        .iter()
        .map(|lane| {
            Ok(Lane {
                id: text(lane.get("id"), "scheduler lane lacks an ID")?.to_owned(),
                lifecycle: redacted_lifecycle(text(
                    lane.get("state"),
                    "scheduler lane lacks a state",
                )?)
                .to_owned(),
            })
        })
        .collect()
}

fn redacted_lifecycle(state: &str) -> &str {
    match state {
        "leased" | "candidate" | "under_review" | "rework" | "accepted" | "integrating" => "active",
        state => state,
    }
}

fn text<'a>(value: Option<&'a Value>, message: &str) -> Result<&'a str, InventoryError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| unavailable(message.to_owned()))
}

fn unavailable(message: String) -> InventoryError {
    InventoryError::InvalidRegistry(message)
}
