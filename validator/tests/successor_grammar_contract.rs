use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
struct SnapshotEntry {
    kind: &'static str,
    mode: u32,
    bytes: Vec<u8>,
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        for _ in 0..128 {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "ultragoal-successor-grammar-{}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => {
                    set_owner_only(&root);
                    return Self { root };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create owner fixture: {error}"),
            }
        }
        panic!("exhausted unique owner fixture names");
    }

    fn snapshot(&self) -> std::collections::BTreeMap<PathBuf, SnapshotEntry> {
        let metadata = fs::symlink_metadata(&self.root).expect("fixture root metadata");
        let mut entries = std::collections::BTreeMap::from([(
            PathBuf::from("."),
            SnapshotEntry {
                kind: "directory",
                mode: mode(&metadata),
                bytes: Vec::new(),
            },
        )]);
        entries.extend(snapshot_tree(&self.root, Path::new("")));
        entries
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("set owner-only fixture");
}

#[cfg(not(unix))]
fn set_owner_only(_: &Path) {}

fn snapshot_tree(
    root: &Path,
    relative: &Path,
) -> std::collections::BTreeMap<PathBuf, SnapshotEntry> {
    let mut entries = std::collections::BTreeMap::new();
    let path = root.join(relative);
    for entry in fs::read_dir(&path).expect("read owned fixture") {
        let entry = entry.expect("fixture entry");
        let name = entry.file_name();
        let child_relative = relative.join(name);
        let child = root.join(&child_relative);
        let metadata = fs::symlink_metadata(&child).expect("fixture metadata");
        let file_type = metadata.file_type();
        let snapshot = if file_type.is_dir() {
            SnapshotEntry {
                kind: "directory",
                mode: mode(&metadata),
                bytes: Vec::new(),
            }
        } else if file_type.is_file() {
            SnapshotEntry {
                kind: "file",
                mode: mode(&metadata),
                bytes: fs::read(&child).expect("fixture file"),
            }
        } else if file_type.is_symlink() {
            SnapshotEntry {
                kind: "symlink",
                mode: mode(&metadata),
                bytes: fs::read_link(&child)
                    .expect("fixture symlink")
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            }
        } else {
            SnapshotEntry {
                kind: "special",
                mode: mode(&metadata),
                bytes: Vec::new(),
            }
        };
        entries.insert(child_relative.clone(), snapshot);
        if file_type.is_dir() {
            entries.extend(snapshot_tree(root, &child_relative));
        }
    }
    entries
}

#[cfg(unix)]
fn mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;

    metadata.mode()
}

#[cfg(not(unix))]
fn mode(_: &fs::Metadata) -> u32 {
    0
}

fn invoke(fixture: &Fixture, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ultragoal"))
        .args(args)
        .env_clear()
        .current_dir(&fixture.root)
        .output()
        .expect("run ultragoal binary")
}

#[test]
fn successor_grammar_catalog_and_help_are_runnable() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();

    let root_help = invoke(&fixture, &["--json", "--help"]);
    assert!(root_help.status.success(), "{root_help:?}");
    assert!(root_help.stderr.is_empty(), "{root_help:?}");
    let root: Value = serde_json::from_slice(&root_help.stdout).expect("root help JSON");
    assert_eq!(root["schema_version"], "harness-ultragoal.cli-help.v1");
    let groups = root["commands"]
        .as_array()
        .expect("catalog commands")
        .iter()
        .filter_map(|command| command["group"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        groups,
        BTreeSet::from([
            "check", "diagnose", "eval", "fit", "inspect", "migrate", "next", "observe", "package",
            "prove",
        ])
    );

    let fit_help = invoke(&fixture, &["--json", "fit", "--help"]);
    assert!(fit_help.status.success(), "{fit_help:?}");
    assert!(fit_help.stderr.is_empty(), "{fit_help:?}");
    let fit: Value = serde_json::from_slice(&fit_help.stdout).expect("fit help JSON");
    assert_eq!(fit["schema_version"], "harness-ultragoal.cli-help.v1");
    assert!(
        fit["commands"]
            .as_array()
            .expect("fit commands")
            .iter()
            .all(|command| command["group"] == "fit"),
        "group help must not widen the catalog"
    );

    let malformed = invoke(&fixture, &["--json", "unsupported-successor-command"]);
    assert!(!malformed.status.success(), "{malformed:?}");
    assert!(malformed.stdout.len() <= 16 * 1024, "{malformed:?}");
    assert!(malformed.stderr.len() <= 16 * 1024, "{malformed:?}");
    let output = [malformed.stdout, malformed.stderr].concat();
    assert!(
        !String::from_utf8_lossy(&output).contains(fixture.root.to_string_lossy().as_ref()),
        "malformed output must not echo the fixture path"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "grammar paths must be zero-write"
    );
}
