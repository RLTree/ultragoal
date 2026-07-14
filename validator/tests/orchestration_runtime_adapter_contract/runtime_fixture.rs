use crate::orchestration::product::command::{
    InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("runtime_fixture/next_root.rs");

include!("runtime_fixture/permit_for_reconciliation.rs");

include!("runtime_fixture/interrupted_heartbeat.rs");
