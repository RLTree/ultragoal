//! Fixture-only process adapter.  This is intentionally separate from public
//! `CommandSpec`: fixture execution has a confined-write lease, while public
//! capture remains read-only and catalog-bound.

#[path = "execute/mod.rs"]
mod execute;
#[path = "permit/mod.rs"]
mod permit;

pub(crate) use permit::{FixtureCaptureAdapter, FixtureCaptureRequest};

use crate::evaluation::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionRuntimeError,
};
use crate::fixture_scheduler::{
    ExpectedOutcome, FixtureKind, FixtureScheduler, FixtureSpec, ResourceKind, RunDisposition,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[path = "bridge/evaluation_bridge.rs"]
mod evaluation_bridge;
#[path = "bridge/scheduled_invocation.rs"]
mod scheduled_invocation;

pub(crate) use scheduled_invocation::*;

#[cfg(test)]
#[path = "../../../../tests/fixture_scheduler_contract/execution_adapter/mod.rs"]
mod adapter_tests;
#[cfg(test)]
#[path = "tests/evaluation_adapter_controls.rs"]
mod evaluation_adapter_controls;
