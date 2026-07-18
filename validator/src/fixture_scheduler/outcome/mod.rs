use super::FixtureScheduleError;
use serde::Serialize;
#[cfg(test)]
use sha2::{Digest, Sha256};

include!("outcome_verdict.rs");

#[cfg(test)]
include!("fixture_execution_record_captured.rs");
