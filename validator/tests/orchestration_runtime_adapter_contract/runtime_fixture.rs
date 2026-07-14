use crate::orchestration::product::command::OrchestrationStateRequest;
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("runtime_fixture/next_root.rs");

pub fn state_request(head: JournalHead, tick: u64) -> OrchestrationStateRequest {
    OrchestrationStateRequest {
        expected_head: head,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
}
