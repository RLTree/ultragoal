use super::runtime_fixtures::*;
use crate::migration::SurfaceStatus;
use crate::migration::product::{
    ApplyOutcomeStatus, JournalPhase, apply_product_plan, issue_apply_authorization,
    recover_product_operation,
};

include!("run_crash_and_recover.rs");

include!(
    "compatibility_recovery_before_effect_refuses_after_deadline_crossing_with_zero_effect.rs"
);
