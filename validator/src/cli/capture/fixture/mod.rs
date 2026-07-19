//! Fixture-only process adapter.  This is intentionally separate from public
//! `CommandSpec`: fixture execution has a confined-write lease, while public
//! capture remains read-only and catalog-bound.

#[path = "execute/mod.rs"]
mod execute;
#[path = "permit/mod.rs"]
mod permit;

pub(crate) use permit::FixtureCaptureAdapter;
#[cfg(test)]
pub(crate) use permit::FixtureCaptureRequest;

#[cfg(test)]
use crate::evaluation::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionRuntimeError,
};
#[cfg(test)]
use crate::fixture_scheduler::{ExpectedOutcome, FixtureKind, FixtureSpec, ResourceKind};
use crate::fixture_scheduler::{FixtureScheduler, RunDisposition};
#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(test)]
#[path = "bridge/evaluation_bridge.rs"]
mod evaluation_bridge;
#[cfg(test)]
#[path = "bridge/scheduled_invocation.rs"]
mod scheduled_invocation;

#[cfg(test)]
pub(crate) use scheduled_invocation::*;

/// Runs one already-scheduled fixture through the crate-controlled confined
/// adapter. The child supplies bytes and an exit status, never an outcome.
pub fn execute_scheduled_fixture(
    scheduler: &mut FixtureScheduler,
    lease_id: &str,
    executable: PathBuf,
    arguments: Vec<OsString>,
    output_limit: usize,
    required_output: Vec<u8>,
) -> Result<RunDisposition, crate::fixture_scheduler::FixtureScheduleError> {
    let fixture = scheduler.run(lease_id)?.fixture.clone();
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        executable,
        arguments,
        output_limit,
        required_output,
    )?;
    scheduler
        .execute_recorded(lease_id, &adapter)
        .map(|(disposition, _)| disposition)
}

#[cfg(test)]
#[path = "../../../../tests/fixture_scheduler_contract/execution_adapter/mod.rs"]
mod adapter_tests;
#[cfg(test)]
#[path = "tests/evaluation_adapter_controls.rs"]
mod evaluation_adapter_controls;
