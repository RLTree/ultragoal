//! Exact request binding and one-use execution transaction.

#[path = "execution_transaction/route.rs"]
pub(super) mod route;

pub(crate) use route::ProductionExecutionOutcome;
pub(super) use route::{ReservedExecution, ValidatedExecution};
