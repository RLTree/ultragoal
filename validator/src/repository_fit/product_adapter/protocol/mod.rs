use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::context::LiveContext;

use super::catalog::{DesiredBundle, compile};
use super::projection::{
    APPLY_PREPARATION_SCHEMA, CLAIM_EFFECT, CandidateProjection, CheckProjection,
    ConflictProjection, DesiredProjection, ExpectedProjection, FitApplyPreparationProjection,
    FitInspectProjection, FitPlanRecord, FitVerificationProjection, INSPECT_SCHEMA,
    InspectionProjection, MutationProjection, ObservedFileProjection, PLAN_SCHEMA, PlanProjection,
    ProvenanceProjection, RollbackEntryProjection, SUPPORT_LIMIT, TargetProjection, VERIFY_SCHEMA,
    VerificationFailureProjection,
};
use super::{AdapterErrorId, FitAdapterError, adapter_error, kernel_error};
use crate::repository_fit::LocalEffects;
use crate::repository_fit::{
    DesiredState, ExpectedContent, FitInspection, FitMode, FitPlan, LocalRepository,
    ObservedDisposition, Ownership, OwnershipProvenance, PlanAuthorization, RepositoryClass,
    digest, inspect, plan, valid_digest, verify,
};

#[path = "apply_preparation.rs"]
mod apply_preparation;
#[path = "plan_input_limit.rs"]
mod plan_input_limit;
#[path = "plan_projection.rs"]
mod plan_projection;
#[path = "plan_record.rs"]
mod plan_record;
#[path = "root_identity.rs"]
mod root_identity;
#[path = "target_inspection.rs"]
mod target_inspection;

pub(crate) use apply_preparation::*;
pub(crate) use plan_input_limit::*;
pub(crate) use plan_projection::*;
pub(crate) use plan_record::*;
pub(crate) use root_identity::*;
pub(crate) use target_inspection::*;
