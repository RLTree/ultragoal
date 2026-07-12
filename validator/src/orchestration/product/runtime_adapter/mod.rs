//! Production-facing adapter over the sealed orchestration command views.
//!
//! Root-owned code supplies the durable workspace, current context, authority,
//! and permit. This module never constructs root authority, issues permits,
//! adopts state, or selects a public command route.

mod action;
mod view;

pub use action::{RuntimeActionOutcome, RuntimeActionRequest, RuntimeActionSource};
pub use view::{CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter};
