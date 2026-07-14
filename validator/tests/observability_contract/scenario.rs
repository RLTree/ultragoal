use super::observability::{EventQuery, EventStore, ExportAdapter, SemanticEvent};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let requested_root = PathBuf::from("/tmp/hul-observability-node-fixtures");
        fs::create_dir_all(&requested_root).expect("create observability fixture root");
        let root = fs::canonicalize(&requested_root).expect("resolve observability fixture root");
        let path = root.join(format!(
            "{}-{}-{serial}",
            std::process::id(),
            safe_label(label)
        ));
        fs::create_dir_all(&path).expect("create bounded observability fixture");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn store_path(&self) -> PathBuf {
        self.path.join("events.jsonl")
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let requested_root = Path::new("/tmp/hul-observability-node-fixtures");
        let Ok(allowed) = fs::canonicalize(requested_root) else {
            return;
        };
        if self.path.starts_with(allowed) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub fn store(dir: &TestDir) -> EventStore {
    EventStore::open_bound(dir.store_path(), "ctx-1", "cand-1", "source-1")
        .expect("open bound store")
}

pub fn query() -> EventQuery {
    EventQuery::new("ctx-1", "cand-1", "source-1").expect("bound query")
}

pub fn event(id: &str, at: u64, sequence: u64, outcome: &str) -> SemanticEvent {
    SemanticEvent::new(
        "ctx-1",
        "cand-1",
        "source-1",
        id,
        at,
        sequence,
        "check.run",
        outcome,
    )
    .expect("bounded event")
}

pub fn tree_snapshot(root: &Path) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    snapshot_entry(root, root, &mut result);
    result
}

fn snapshot_entry(root: &Path, path: &Path, result: &mut BTreeMap<String, String>) {
    let metadata = fs::symlink_metadata(path).expect("snapshot metadata");
    let relative = path
        .strip_prefix(root)
        .expect("snapshot prefix")
        .to_string_lossy()
        .to_string();
    if metadata.is_dir() {
        result.insert(relative, format!("dir:{}", metadata.len()));
        let mut children: Vec<_> = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry").path())
            .collect();
        children.sort();
        for child in children {
            snapshot_entry(root, &child, result);
        }
    } else if metadata.is_file() {
        let bytes = fs::read(path).expect("snapshot file");
        result.insert(
            relative,
            format!("file:{}:{:x}", bytes.len(), Sha256::digest(bytes)),
        );
    } else {
        result.insert(relative, "special".to_owned());
    }
}

#[derive(Clone, Copy)]
pub enum AdapterMode {
    Ok,
    Outage,
    Delay,
    Duplicate,
    Partial,
    ReconcilePartial,
}

pub struct MockAdapter {
    pub candidate_id: String,
    pub available: bool,
    pub redacted_only: bool,
    pub mode: AdapterMode,
    pub exported: Vec<SemanticEvent>,
    pub export_calls: usize,
}

impl MockAdapter {
    pub fn new(mode: AdapterMode) -> Self {
        Self {
            candidate_id: "cand-1".to_owned(),
            available: true,
            redacted_only: true,
            mode,
            exported: Vec::new(),
            export_calls: 0,
        }
    }
}

impl ExportAdapter for MockAdapter {
    fn adapter_id(&self) -> &str {
        "mock-export"
    }
    fn bound_candidate_id(&self) -> &str {
        &self.candidate_id
    }
    fn is_available(&self) -> bool {
        self.available
    }
    fn accepts_redacted_only(&self) -> bool {
        self.redacted_only
    }

    fn export(
        &mut self,
        events: &[SemanticEvent],
        _timeout: Duration,
    ) -> Result<Vec<String>, String> {
        self.export_calls += 1;
        self.exported = events.to_vec();
        match self.mode {
            AdapterMode::Outage => Err("synthetic backend secret detail".to_owned()),
            AdapterMode::Delay => {
                std::thread::sleep(Duration::from_millis(20));
                Ok(ids(events))
            }
            AdapterMode::Duplicate => {
                let mut values = ids(events);
                values.push(values[0].clone());
                Ok(values)
            }
            AdapterMode::Partial => Ok(Vec::new()),
            AdapterMode::Ok | AdapterMode::ReconcilePartial => Ok(ids(events)),
        }
    }

    fn reconcile(
        &mut self,
        _candidate_id: &str,
        event_ids: &[String],
        _timeout: Duration,
    ) -> Result<Vec<String>, String> {
        if matches!(self.mode, AdapterMode::ReconcilePartial) {
            Ok(Vec::new())
        } else {
            Ok(event_ids.to_vec())
        }
    }
}

fn ids(events: &[SemanticEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.event_id().to_owned())
        .collect()
}

fn safe_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}
