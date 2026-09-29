use crate::observability::{
    EventQuery, EventStore, ExplicitExportRequest, ExportAdapter, SemanticEvent, SemanticEventInput,
};
use std::fs::{self, File};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

mod cleanup;
mod contention;
mod deadline;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestStore {
    root: std::path::PathBuf,
    store: EventStore,
}

impl TestStore {
    fn seeded(label: &str) -> Self {
        let temporary_root =
            fs::canonicalize(std::env::temp_dir()).expect("resolve identity test temp root");
        let root = temporary_root.join(format!(
            "ultragoal-observability-identity-{label}-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create identity test directory");
        let store =
            EventStore::open_bound(root.join("events.jsonl"), "context", "candidate", "source")
                .expect("open identity test store");
        assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
        Self { root, store }
    }

    fn query(&self) -> EventQuery {
        EventQuery::new("context", "candidate", "source").expect("build identity test query")
    }

    fn store_bytes(&self) -> Vec<u8> {
        fs::read(self.store.path()).expect("read identity test store")
    }

    fn open_store_file(&self) -> File {
        File::options()
            .read(true)
            .write(true)
            .open(self.store.path())
            .expect("open identity test store file")
    }

    fn remove(self) {
        fs::remove_dir_all(&self.root).expect("remove identity test directory");
    }
}

fn event(id: &str, observed_at: u64, sequence: u64, outcome: &str) -> SemanticEvent {
    SemanticEvent::new(SemanticEventInput {
        context_id: "context".to_owned(),
        candidate_id: "candidate".to_owned(),
        source_id: "source".to_owned(),
        event_id: id.to_owned(),
        observed_at_unix_ms: observed_at,
        sequence,
        operation: "check.run".to_owned(),
        outcome: outcome.to_owned(),
    })
    .expect("build identity test event")
}

fn identity_holder(
    store: &EventStore,
) -> (
    mpsc::Sender<()>,
    std::thread::JoinHandle<Result<(), mpsc::RecvError>>,
) {
    let value = Arc::clone(&store.identity.value);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _guard = value.lock().expect("lock identity test mutex");
        ready_tx.send(()).expect("signal identity test holder");
        release_rx.recv()
    });
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("identity test holder did not start");
    (release_tx, holder)
}

fn assert_identity_timeout<T>(store: &EventStore, operation: impl FnOnce() -> Result<T, String>) {
    let (release, holder) = identity_holder(store);
    let started = Instant::now();
    let error = operation().err().expect("identity contention must fail");
    let elapsed = started.elapsed();
    release.send(()).expect("release identity test holder");
    holder
        .join()
        .expect("identity test holder panicked")
        .expect("identity test holder cancelled");
    assert_eq!(error, "observe-store-lock-timeout");
    assert!(
        elapsed
            >= crate::observability::locking::STORE_LOCK_TIMEOUT
                .saturating_sub(Duration::from_millis(50)),
        "identity timeout returned too early: {elapsed:?}"
    );
    assert!(
        elapsed <= crate::observability::locking::STORE_LOCK_TIMEOUT + Duration::from_millis(750),
        "identity timeout was unbounded: {elapsed:?}"
    );
}

struct NoEffectAdapter {
    calls: usize,
}

impl ExportAdapter for NoEffectAdapter {
    fn adapter_id(&self) -> &str {
        "identity-test-adapter"
    }
    fn bound_candidate_id(&self) -> &str {
        "candidate"
    }
    fn is_available(&self) -> bool {
        true
    }
    fn accepts_redacted_only(&self) -> bool {
        true
    }
    fn export(
        &mut self,
        _events: &[SemanticEvent],
        _timeout: Duration,
    ) -> Result<Vec<String>, String> {
        self.calls += 1;
        Ok(Vec::new())
    }
    fn reconcile(
        &mut self,
        _candidate_id: &str,
        _event_ids: &[String],
        _timeout: Duration,
    ) -> Result<Vec<String>, String> {
        self.calls += 1;
        Ok(Vec::new())
    }
}
