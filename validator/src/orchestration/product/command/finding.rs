use super::super::{ProductError, ProductSnapshot, RootOperation};
use super::RootActionRequest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    AmbiguousEffect,
    PendingIntegration,
    StaleLease,
    ExpiredLease,
    OrphanedLease,
    RootInterrupted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationFinding {
    pub finding_id: String,
    pub kind: FindingKind,
    pub code: String,
    pub lease_id: Option<String>,
    pub operation_id: Option<String>,
    pub cause: String,
    pub repair: String,
    pub rerun: String,
    pub effect: String,
    pub ceiling: String,
}

pub(crate) fn build_findings(
    snapshot: &ProductSnapshot,
    pending: &BTreeMap<String, String>,
) -> Result<Vec<OrchestrationFinding>, ProductError> {
    let mut findings = Vec::new();
    for operation in &snapshot.recovery.ambiguous_operations {
        findings.push(finding(
            snapshot,
            FindingKind::AmbiguousEffect,
            pending.get(operation).cloned(),
            Some(operation.clone()),
        )?);
    }
    if snapshot.recovery.pending_integration_id.is_some() {
        findings.push(finding(
            snapshot,
            FindingKind::PendingIntegration,
            None,
            snapshot.recovery.pending_integration_id.clone(),
        )?);
    }
    for (kind, leases) in [
        (
            FindingKind::StaleLease,
            &snapshot.recovery.stale_binding_leases,
        ),
        (FindingKind::ExpiredLease, &snapshot.recovery.expired_leases),
        (
            FindingKind::OrphanedLease,
            &snapshot.recovery.orphaned_leases,
        ),
    ] {
        for lease in leases {
            findings.push(finding(snapshot, kind, Some(lease.clone()), None)?);
        }
    }
    if snapshot.recovery.interrupted_root {
        findings.push(finding(snapshot, FindingKind::RootInterrupted, None, None)?);
    }
    findings.sort_by_key(|finding| (finding.kind, finding.finding_id.clone()));
    Ok(findings)
}

pub(crate) fn action_matches_finding(
    action: &RootActionRequest,
    finding: &OrchestrationFinding,
) -> bool {
    match action.operation {
        RootOperation::Reconcile => {
            finding.kind == FindingKind::AmbiguousEffect
                && finding.operation_id == action.target.operation_id
        }
        RootOperation::Recover => false,
        RootOperation::Resume => finding.kind == FindingKind::RootInterrupted,
    }
}

fn finding(
    snapshot: &ProductSnapshot,
    kind: FindingKind,
    lease_id: Option<String>,
    operation_id: Option<String>,
) -> Result<OrchestrationFinding, ProductError> {
    let (code, cause, repair, rerun) = finding_text(kind);
    let identity = serde_json::to_vec(&(
        kind,
        &snapshot.binding,
        &snapshot.journal_head,
        &lease_id,
        &operation_id,
    ))
    .map_err(|_| ProductError::StaleCandidate)?;
    Ok(OrchestrationFinding {
        finding_id: format!("sha256:{:x}", Sha256::digest(identity)),
        kind,
        code: code.to_owned(),
        lease_id,
        operation_id,
        cause: cause.to_owned(),
        repair: repair.to_owned(),
        rerun: rerun.to_owned(),
        effect: "read".to_owned(),
        ceiling: "orchestration command and journey claims remain withheld".to_owned(),
    })
}

fn finding_text(kind: FindingKind) -> (&'static str, &'static str, &'static str, &'static str) {
    match kind {
        FindingKind::AmbiguousEffect => (
            "HUL-ORCH-STATE-001",
            "an effect intent has no authoritative applied or not-applied outcome",
            "obtain independent effect evidence and request root reconciliation",
            "ultragoal --json diagnose",
        ),
        FindingKind::PendingIntegration => (
            "HUL-ORCH-STATE-002",
            "a root integration has not reached one reconciled disposition",
            "re-observe the exact root integration intent before candidate rebound",
            "ultragoal --json inspect",
        ),
        FindingKind::StaleLease => (
            "HUL-ORCH-STATE-003",
            "an active lease is bound to a different candidate",
            "cancel or replace the stale lease under root authority",
            "ultragoal --json inspect",
        ),
        FindingKind::ExpiredLease => (
            "HUL-ORCH-STATE-004",
            "an active lease exceeded its heartbeat deadline",
            "issue a new candidate-bound lease after root review",
            "ultragoal --json inspect",
        ),
        FindingKind::OrphanedLease => (
            "HUL-ORCH-STATE-005",
            "an active lease has no independently observed live worker",
            "reconcile the worker result or replace the orphaned lease",
            "ultragoal --json inspect",
        ),
        FindingKind::RootInterrupted => (
            "HUL-ORCH-STATE-006",
            "the durable journal records an interrupted root session",
            "validate the typed resume request and issue a short-lived root permit",
            "ultragoal --json next",
        ),
    }
}
