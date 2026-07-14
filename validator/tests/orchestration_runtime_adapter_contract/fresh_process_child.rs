use super::runtime_fixture::*;
use crate::orchestration::product::command::{
    CommandProjection, InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

include!("fresh_process_child/child_env.rs");

include!("fresh_process_child/reconcile_current.rs");
