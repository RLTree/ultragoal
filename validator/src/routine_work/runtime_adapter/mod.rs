//! Internal effect-request protocol for routine execution.
//!
//! This module prepares opaque intents, binds read-only evidence expectations,
//! and reconciles already-observed outcomes. It owns no workspace-effect grant
//! and performs no workspace, receipt, cache, telemetry, network, or external
//! write.

mod execution_authority;
mod mediator;
mod production;

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::context::{EffectClass, LiveContext, ToolCapability};

use super::digest::{digest_of, framed, valid};
use super::{
    DependencyResult, DirtySnapshot, ImpactGraph, PlanMode, PlannedCheck, RepoPath,
    ReportDisposition, ReportRecord, ReportStatus, ReuseExpectation, RoutineBinding, RoutineError,
    RoutineErrorId, RoutinePlan, RoutineReport, RunOutcome, reconcile_report,
};

use execution_authority::RoutineReadSource;
pub(crate) use execution_authority::{
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineEffectIntent, RoutineEffectRequest,
    RoutineInvocationSpec, RoutineMediatedExpectation, RoutineMediatedIntent,
    RoutineMediatedOutcome, RoutineMediatedWitness, RoutineMediationAuthority,
    RoutineMediationBatch, RoutineNoOpProjection,
};
pub(crate) use mediator::{
    RoutineCancellation, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutineNodeMediation, RoutineReuseInput, RoutineRootGrant, mediate_prepared_routine_execution,
};
#[cfg(test)]
pub(crate) use mediator::{
    TestProcessSetupFailure, set_test_mediator_finish_failure, set_test_mediator_post_spawn_hook,
    set_test_mediator_pre_spawn_hook, set_test_output_capture_hook, set_test_process_setup_failure,
    set_test_read_source_capture_hook, test_spawn_count,
};
pub(crate) use production::{
    ProductionRoutineIssuer, RoutineRecoveryAuthority,
    mediate_prepared_routine_execution_production,
};

#[path = "execution_preparation.rs"]
mod execution_preparation;
#[path = "execution_reconciliation.rs"]
mod execution_reconciliation;
#[path = "invocation_binding.rs"]
mod invocation_binding;
#[path = "mediation_issuance.rs"]
mod mediation_issuance;
#[path = "output_scope_binding.rs"]
mod output_scope_binding;
#[path = "runner_binding.rs"]
mod runner_binding;
#[path = "selection_limit.rs"]
mod selection_limit;

pub(crate) use execution_preparation::*;
pub(crate) use execution_reconciliation::*;
pub(crate) use invocation_binding::*;
pub(crate) use mediation_issuance::*;
pub(crate) use output_scope_binding::*;
pub(crate) use runner_binding::*;
pub(crate) use selection_limit::*;
