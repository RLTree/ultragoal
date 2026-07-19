use super::EvaluationRun;
use super::{EvaluationError, EvaluationTaskResult, FailureCase, PromotionDecision};
use crate::fixture_scheduler::FixtureExecutionRecord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

include!("max_configuration_value_bytes.rs");

include!("evaluation_event_kind.rs");
