//! Root-owned repository-fit public adapter surface.
//!
//! Read routes remain projection-only. Apply consumes one exact accepted plan,
//! persists restart authority in preprovisioned owner-only host state, and then
//! enters the sealed repository-fit production kernel.

use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::repository_fit::{
    AdapterErrorId, FitAdapterError, FitPlanScope, PreparedFitApply, inspect_target, plan_target,
    plan_target_for_scope, prepare_apply_request, verify_target,
};
use std::path::{Path, PathBuf};

pub(crate) fn repository_fit_authority_migration_diagnosis(
    read_context: &LiveContext,
    home: Option<&Path>,
) -> Option<RuntimeOutcome> {
    let admission = match authority::assess_migration_admission(home?) {
        Ok(Some(admission)) => admission,
        Ok(None) => return None,
        Err(()) => {
            let machine = serde_json::to_vec(&serde_json::json!({
                "schema_version": "RepositoryFitAuthorityMigrationAdmission-v1",
                "status": "authority_state_invalid",
                "authority_status": "unavailable",
                "migration_effect": "none",
                "migration_authorized": false,
                "quarantine_plan": null,
                "next_action": "preserve the complete state owner and require authenticated repair evidence before any migration planning",
                "claim_effect": "none",
                "support_limit": "read-only invalid-state classification only; no state details, migration plan, migration apply, quarantine effect, bootstrap, installed journey, product, or release claim"
            })).ok()?;
            if read_context.revalidate().is_err() || !super::public_output_allowed(machine.len()) {
                return None;
            }
            return Some(RuntimeOutcome::payload(
                ExitClass::ActionableFinding,
                machine,
                "repository-fit authority migration admission status=authority_state_invalid effect=none authorized=false".to_owned(),
            ));
        }
    };
    let machine = serde_json::to_vec(&serde_json::json!({
        "schema_version": "RepositoryFitAuthorityMigrationAdmission-v1",
        "status": admission.status,
        "authority_status": admission.authority_status,
        "generation": admission.generation,
        "event_count": admission.event_count,
        "reservation_count": admission.reservation_count,
        "nonterminal_reservation_count": admission.nonterminal_reservation_count,
        "pending_envelope_count": admission.pending_envelope_count,
        "process_lock_count": admission.process_lock_count,
        "stored_root": admission.stored_root,
        "current_root": admission.current_root,
        "stored_lock": admission.stored_lock,
        "current_lock": admission.current_lock,
        "migration_effect": "none",
        "migration_authorized": false,
        "quarantine_plan": admission.quarantine_plan.as_ref().map(|plan| serde_json::json!({
            "schema_version": "RepositoryFitAuthorityQuarantinePlan-v1",
            "plan_id": plan.plan_id,
            "status": "review_required",
            "source_inventory_sha256": plan.source_inventory_sha256,
            "ledger_sha256": plan.ledger_sha256,
            "authority_id": plan.authority_id,
            "ledger_head_sha256": plan.ledger_head_sha256,
            "source_owner": plan.source_owner,
            "quarantine_owner": plan.quarantine_owner,
            "target_format": plan.target_format,
            "strategy": plan.strategy,
            "apply_capability": plan.apply_capability,
            "operations": plan.operations,
            "rollback": plan.rollback,
            "precondition": "same authenticated complete source inventory under exclusive parent custody; quarantine destination absent",
            "migration_effect": "none",
            "migration_authorized": false,
            "claim_effect": "none"
        })),
        "next_action": admission.next_action,
        "claim_effect": "none",
        "support_limit": "read-only authenticated repository-fit authority drift diagnosis and deterministic whole-owner quarantine planning; no migration apply, quarantine effect, bootstrap, installed journey, product, or release claim"
    })).ok()?;
    if read_context.revalidate().is_err() || !super::public_output_allowed(machine.len()) {
        return None;
    }
    Some(RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        machine,
        format!(
            "repository-fit authority migration admission status={} effect=none authorized=false",
            admission.status
        ),
    ))
}

mod authority;
pub(super) mod external_plan_file;
#[path = "invocation_errors.rs"]
mod invocation_errors;
#[path = "plan_input_limit.rs"]
mod plan_input_limit;

pub(crate) use invocation_errors::*;
pub(crate) use plan_input_limit::*;
