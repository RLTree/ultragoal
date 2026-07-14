use super::fixture::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ResumeRequest,
};

#[test]
fn inner_authority_and_direct_executors_are_compile_private() {
    let root = privacy_consumer();
    assert_consumer_rejected(
        &root,
        "inner-type",
        "use ultragoal::orchestration::product::RootAuthority; fn main() {}",
        "private",
    );
    assert_consumer_rejected(
        &root,
        "extract",
        "use ultragoal::orchestration::product::ProductionRootAuthority; fn probe(a: &ProductionRootAuthority) { let _ = &a.authority; } fn main() {}",
        "private",
    );
    assert_consumer_rejected(
        &root,
        "clone",
        "use ultragoal::orchestration::product::ProductionRootAuthority; fn probe(a: &ProductionRootAuthority) { let _ = a.authority.clone(); } fn main() {}",
        "private",
    );
    assert_consumer_rejected(&root, "direct-action", DIRECT_ACTION, "private");
    assert_consumer_rejected(&root, "direct-reconcile", DIRECT_RECONCILE, "private");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cross_authority_attempt_and_replay_cannot_bypass_one_use_ledger() {
    let (journal, head) = interrupted_root("sealed-route-journal");
    let origin_root = TestRoot::new("sealed-route-origin", 0o700);
    let other_root = TestRoot::new("sealed-route-other", 0o700);
    let (action, permit, origin) = issue_resume(&origin_root, &journal, head.clone(), 2);
    let other =
        ProductionRootAuthority::open_or_initialize(other_root.path(), root_actor()).unwrap();
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    let origin_before = recursive_fingerprint(origin_root.path());
    let other_before = recursive_fingerprint(other_root.path());
    assert_eq!(
        adapter
            .execute_production_action(
                &other,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(origin_root.path()), origin_before);
    assert_eq!(recursive_fingerprint(other_root.path()), other_before);
    assert_eq!(
        origin.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
    adapter
        .execute_production_action(
            &origin,
            RuntimeActionSource::Current(&view),
            &action,
            &permit,
            &request,
        )
        .unwrap();
    let committed = recursive_fingerprint(origin_root.path());
    assert_eq!(
        adapter
            .execute_production_action(
                &origin,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityReplay
    );
    assert_eq!(recursive_fingerprint(origin_root.path()), committed);
}

fn privacy_consumer() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "orchestration-authority-privacy-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"authority-privacy\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n\n[dependencies]\nultragoal = {{ path = {:?} }}\n",
            manifest
        ),
    )
    .unwrap();
    root
}

fn assert_consumer_rejected(root: &PathBuf, label: &str, source: &str, expected: &str) {
    fs::write(root.join("src/main.rs"), source).unwrap();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet"])
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{label} unexpectedly compiled");
    assert!(stderr.contains(expected), "{label}: {stderr}");
}

const DIRECT_ACTION: &str = r#"
use ultragoal::orchestration::product::command::RootActionRequest;
use ultragoal::orchestration::product::runtime_adapter::{OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource};
use ultragoal::orchestration::product::RootPermit;
fn probe<'a>(adapter: &OrchestrationRuntimeAdapter<'a>, source: RuntimeActionSource<'_>, action: &RootActionRequest, permit: &RootPermit, request: &RuntimeActionRequest) {
    let _ = adapter.execute_action(source, action, panic!(), permit, request);
}
fn main() {}
"#;

const DIRECT_RECONCILE: &str = r#"
use ultragoal::orchestration::product::command::RootActionRequest;
use ultragoal::orchestration::product::runtime_adapter::{CurrentRuntimeView, OrchestrationRuntimeAdapter};
use ultragoal::orchestration::product::{ReconcileRequest, RootPermit};
fn probe<'a>(adapter: &OrchestrationRuntimeAdapter<'a>, view: &CurrentRuntimeView, action: &RootActionRequest, permit: &RootPermit, request: &ReconcileRequest) {
    let _ = adapter.execute_reconcile(view, action, panic!(), permit, request);
}
fn main() {}
"#;
