use super::catalog::{DependencyStatus, FactAuthority};
use super::types::{
    AuthorityRequirement, CeilingReduction, Finding, FindingSeverity, FindingSource, Repair,
    RepairTarget, RepairTargetKind, Scope,
};
use crate::context::EffectClass;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Serialize)]
struct FindingIdentity<'a> {
    code: &'a str,
    severity: FindingSeverity,
    source: &'a FindingSource,
    scope: &'a Scope,
    dependency_ids: &'a BTreeSet<String>,
    cause: &'a str,
    repair: &'a Repair,
    ceiling_reductions: &'a [CeilingReduction],
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finding(
    code: impl Into<String>,
    severity: FindingSeverity,
    source: FindingSource,
    scope: Scope,
    dependency_ids: BTreeSet<String>,
    cause: impl Into<String>,
    repair: Repair,
    mut reductions: Vec<CeilingReduction>,
) -> Finding {
    let code = code.into();
    let cause = cause.into();
    reductions.sort_by(|a, b| (&a.claim_id, &a.dimensions).cmp(&(&b.claim_id, &b.dimensions)));
    reductions.dedup();
    let affected_claims = reductions
        .iter()
        .map(|item| item.claim_id.clone())
        .collect::<BTreeSet<_>>();
    let bytes = serde_json::to_vec(&FindingIdentity {
        code: &code,
        severity,
        source: &source,
        scope: &scope,
        dependency_ids: &dependency_ids,
        cause: &cause,
        repair: &repair,
        ceiling_reductions: &reductions,
    })
    .expect("serializing owned state finding cannot fail");
    Finding {
        finding_id: format!("sha256:{:x}", Sha256::digest(bytes)),
        code,
        severity,
        source,
        scope,
        dependency_ids,
        cause,
        effect: repair.effect,
        authority: repair.authority,
        repair,
        affected_claims,
        ceiling_reductions: reductions,
    }
}

pub(crate) fn policy_repair(code: &str) -> Repair {
    Repair {
        repair_id: format!("repair-state-policy-{code}"),
        target: RepairTarget {
            kind: RepairTargetKind::Configuration,
            id: "dependency-action-catalog".to_owned(),
        },
        summary: format!("Root must reconcile state policy defect {code}"),
        effect: EffectClass::PlannedWrite,
        authority: AuthorityRequirement::Root,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::from(["state-policy-projections".to_owned()]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}

pub(crate) fn contradiction_repair(dependency_id: &str) -> Repair {
    Repair {
        repair_id: format!("reobserve-{dependency_id}"),
        target: RepairTarget {
            kind: RepairTargetKind::Dependency,
            id: dependency_id.to_owned(),
        },
        summary: "Reobserve the contradictory dependency from its named authority sources"
            .to_owned(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::Root,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::from([dependency_id.to_owned()]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}

pub(crate) fn dependency_severity(status: DependencyStatus) -> FindingSeverity {
    match status {
        DependencyStatus::Satisfied => FindingSeverity::Info,
        DependencyStatus::Missing => FindingSeverity::Error,
        DependencyStatus::BlockedAuthority => FindingSeverity::Blocked,
        DependencyStatus::Unsupported => FindingSeverity::Warning,
    }
}

pub(crate) fn dependency_code(status: DependencyStatus) -> &'static str {
    match status {
        DependencyStatus::Satisfied => "dependency-satisfied",
        DependencyStatus::Missing => "dependency-missing",
        DependencyStatus::BlockedAuthority => "dependency-blocked-authority",
        DependencyStatus::Unsupported => "dependency-unsupported",
    }
}

pub(crate) fn authority_name(authority: FactAuthority) -> &'static str {
    match authority {
        FactAuthority::DirectProbe => "direct-probe",
        FactAuthority::LiveContext => "live-context",
        FactAuthority::AuthorityCatalog => "authority-catalog",
        FactAuthority::ExternalAuthority => "external-authority",
    }
}
