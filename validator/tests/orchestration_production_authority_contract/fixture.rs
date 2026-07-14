use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::orchestration::product::command::{
    InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    ProductContext, ProductWorkspace, ProductionRootAuthority, ResumeRequest, RootPermit,
};
use ultragoal::orchestration::*;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
pub const CHILD_ENV: &str = "ULTRAGOAL_ORCHESTRATION_AUTHORITY_CHILD";
pub const CHILD_TEST: &str =
    "orchestration_production_authority_contract::race::production_authority_child";

pub struct TestRoot(PathBuf);

impl TestRoot {
    pub fn new(label: &str, mode: u32) -> Self {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "orchestration-production-authority-{label}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

include!("fixture/scenario.rs");
include!("fixture/execution_permit.rs");

pub fn inspect_resume(
    journal: &TestRoot,
    head: JournalHead,
    tick: u64,
) -> (ProductContext, ProductWorkspace, RootActionRequest) {
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    (context, workspace, action)
}

pub fn issue_resume(
    authority_root: &TestRoot,
    journal: &TestRoot,
    head: JournalHead,
    tick: u64,
) -> (RootActionRequest, RootPermit, ProductionRootAuthority) {
    let (context, workspace, action) = inspect_resume(journal, head.clone(), tick);
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(
            &authority,
            RuntimeActionSource::Current(&view),
            &action,
            tick + 10,
        )
        .unwrap();
    (action, permit, authority)
}

include!("fixture/process.rs");

pub fn recursive_fingerprint(root: &Path) -> Vec<(String, u64, u32, u64, u64, String)> {
    let mut rows = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    rows.sort();
    rows.into_iter()
        .map(|path| {
            let metadata = fs::symlink_metadata(&path).unwrap();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                metadata.len(),
                metadata.permissions().mode() & 0o7777,
                metadata.dev(),
                metadata.ino(),
                format!("sha256:{:x}", Sha256::digest(fs::read(&path).unwrap())),
            )
        })
        .collect()
}
