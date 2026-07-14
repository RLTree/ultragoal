#[cfg(test)]
use super::surface_codec::encode_surface;
use super::surface_codec::{
    PackageBinding, SURFACE_LIMIT, decode_surface, surface_tree, surface_tree_sha256,
};
use super::transaction_plan::{
    DarwinHostTransactionPlan, SurfaceStep, plan_sha256_from_parts, validate_operation,
};
use super::{
    DarwinHostError, DarwinHostErrorId, DarwinHostOperation, DarwinHostSnapshot, DarwinHostSurface,
    DarwinHostTransactionDisposition, DarwinHostTransactionReport, DarwinSurfaceObservation,
    DarwinSurfaceStatus, SUPPORTED_MARKETPLACE, SUPPORTED_PLUGIN_ID, SUPPORTED_VERSION,
    Sha256Digest,
};
use crate::distribution::{
    ConfinedRoot, DistributionError, DistributionErrorId, MaterializeEffects, PackageSnapshot,
    ScopedTree, TreeObject, tree_sha256,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub(super) use super::transaction_codec::{
    decode_journal, decode_lineage, decode_lineage_anchor, journal_bytes, lineage_anchor_bytes,
    lineage_bytes,
};

const JOURNAL_PATH: &str = "host-lifecycle/transaction";
const LINEAGE_PATH: &str = "host-lifecycle/lineage";
const LINEAGE_ANCHOR_PATH: &str = "host-lifecycle/lineage-anchor";
const TREE_ENTRY_LIMIT: usize = 2;

#[derive(Clone)]
pub struct DarwinHostTransactionAdapter {
    root: ConfinedRoot,
    root_id: String,
    lineage_floor: Arc<Mutex<Option<LineageFloor>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinTestPoint {
    BeforeJournalReservation,
    AfterJournalReservation,
    BeforeSurface(DarwinHostSurface),
    BeforeCompareExchange(DarwinHostSurface),
    AfterSurface(DarwinHostSurface),
    AfterProgress(DarwinHostSurface),
    AfterCancellationClaim,
    BeforeCancellationRemoval,
    AfterLineageBeforeAnchor,
    BeforeJournalRemoval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinTestControl {
    Continue,
    Interrupt,
}

#[derive(Clone, Debug)]
struct RawTree {
    bytes: Vec<u8>,
    tree_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalBefore {
    surface: DarwinHostSurface,
    tree_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
enum JournalPhase {
    Pending,
    Applying { step: usize, generation: u64 },
    Cancelling { generation: u64 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TransactionJournal {
    schema: String,
    root_id: String,
    generation: u64,
    lineage_before_sha256: Option<String>,
    plan_sha256: String,
    operation: DarwinHostOperation,
    target: PackageBinding,
    marketplace: String,
    next_step: usize,
    phase: JournalPhase,
    before: Vec<JournalBefore>,
}

#[derive(Clone, Debug)]
struct JournalSnapshot {
    record: TransactionJournal,
    tree_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum LineageOutcome {
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TransactionLineage {
    schema: String,
    root_id: String,
    generation: u64,
    previous_lineage_sha256: Option<String>,
    plan_sha256: String,
    operation: DarwinHostOperation,
    target: PackageBinding,
    prior: Option<PackageBinding>,
    marketplace: String,
    steps: Vec<LineagePlanStep>,
    outcome: LineageOutcome,
    terminal: Vec<JournalBefore>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LineagePlanStep {
    surface: DarwinHostSurface,
    desired_tree_sha256: Option<String>,
    prior_tree_sha256: Option<String>,
}

#[derive(Clone, Debug)]
struct LineageSnapshot {
    record: TransactionLineage,
    tree_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TransactionLineageAnchor {
    schema: String,
    root_id: String,
    generation: u64,
    lineage_sha256: String,
}

#[derive(Clone, Debug)]
struct LineageAnchorSnapshot {
    record: TransactionLineageAnchor,
    tree_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LineageFloor {
    generation: u64,
    tree_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JournalAuthority {
    Active,
    Terminal(LineageOutcome),
}

mod execution;
mod journal;
mod lineage;
mod product_api;
mod recovery;
mod state_validation;
#[path = "surface/effect.rs"]
mod surface_effect;
#[path = "surface/observation.rs"]
mod surface_observation;
mod test_surface_seed;

mod record_validation;
use record_validation::*;
