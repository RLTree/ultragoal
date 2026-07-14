use super::ProductError;
use crate::orchestration::Binding;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

include!("authority_schema.rs");

mod production;
#[cfg(test)]
pub(crate) use production::{
    root_authority_for_test, RootActionPermitIssuance, RootAuthority, RootReconcilePermitIssuance,
};
pub use production::{PermitReplayState, ProductionRootAuthority};
pub(crate) use production::{ProductionExecutionOutcome, ReservationObservation};

include!("issue_action_permit_for_test.rs");
