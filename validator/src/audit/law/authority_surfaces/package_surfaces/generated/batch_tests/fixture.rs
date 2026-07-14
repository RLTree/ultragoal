use super::super::State;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const FIRST: &str = "docs/generated/observability/command-inventory.json";
pub(super) const SECOND: &str = "examples/generated/review-debrief.json";
pub(super) const INPUT: &str = "inputs/source.txt";
pub(super) const REGISTRY: &str = "migration/generated-surface-authority.json";
pub(super) const SECRET: &str = "SECRET_CANARY must never escape through an error";

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("docs output directory");
    fs::create_dir_all(root.join("examples/generated")).expect("example output directory");
    fs::create_dir_all(root.join("inputs")).expect("input directory");
    fs::create_dir_all(root.join("migration")).expect("migration directory");
    root
}

fn digest(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string()
}

fn retained(output: &str, bytes: &[u8]) -> Value {
    json!({
        "disposition": "retained_context",
        "output": output,
        "sha256": digest(bytes),
        "reason": "preserved predecessor context",
        "replacement_targets": ["HCT-OBSERVE"],
        "preserve": true,
        "physical_deletion_authorized": false
    })
}

fn write_registry(root: &Path, surfaces: Vec<Value>) {
    fs::write(
        root.join(REGISTRY),
        serde_json::to_vec(&json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": surfaces
        }))
        .expect("registry bytes"),
    )
    .expect("registry");
}

pub(super) fn retained_fixture(label: &str) -> PathBuf {
    let root = root(label);
    fs::write(root.join(FIRST), b"first retained context").expect("first output");
    fs::write(root.join(SECOND), b"second retained context").expect("second output");
    write_registry(
        &root,
        vec![
            retained(FIRST, b"first retained context"),
            retained(SECOND, b"second retained context"),
        ],
    );
    root
}

pub(super) fn canonical_fixture(label: &str) -> PathBuf {
    let root = root(label);
    fs::write(root.join(INPUT), b"canonical input").expect("input");
    let input_row = json!({"path": INPUT, "sha256": digest(b"canonical input")});
    fs::write(
        root.join(FIRST),
        serde_json::to_vec(&json!({
            "_meta": {
                "generator": "HCT-INVENTORY",
                "inputs": [input_row.clone()],
                "recipe": "input-digest-index-v1"
            },
            "entries": [input_row]
        }))
        .expect("canonical output"),
    )
    .expect("first output");
    fs::write(root.join(SECOND), b"second retained context").expect("second output");
    write_registry(
        &root,
        vec![
            json!({
                "disposition": "canonical_projection",
                "output": FIRST,
                "generator": "HCT-INVENTORY",
                "recipe": "input-digest-index-v1",
                "inputs": [INPUT]
            }),
            retained(SECOND, b"second retained context"),
        ],
    );
    root
}

pub(super) fn inventory() -> BTreeSet<String> {
    [FIRST.to_string(), SECOND.to_string()]
        .into_iter()
        .collect()
}

pub(super) fn file_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = fs::read_dir(current)
            .expect("read fixture tree")
            .map(|entry| entry.expect("fixture entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("fixture metadata");
            if metadata.is_dir() {
                visit(root, &path, rows);
            } else if metadata.is_file() {
                rows.insert(
                    path.strip_prefix(root)
                        .expect("relative fixture path")
                        .to_path_buf(),
                    fs::read(path).expect("fixture bytes"),
                );
            }
        }
    }

    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}

pub(super) fn assert_whole_batch_invalid(root: &Path) {
    let state = State::build(root, &inventory());
    for path in [FIRST, SECOND] {
        assert_eq!(
            state.row(path).authority_level,
            "invalid_generated_authority",
            "partial authority escaped for {path}"
        );
    }
    let failures = state.failures();
    assert_eq!(
        failures.len(),
        1,
        "one fixed batch failure expected: {failures:?}"
    );
    assert_eq!(failures[0].0, "generated-disposition-batch");
    assert!(
        failures[0]
            .1
            .contains("failure_class=generated_disposition_batch_invalid")
    );
    assert!(failures[0].1.contains("invalid_generated_path_count=2"));
    let document = serde_json::to_string(&failures).expect("failure document");
    assert!(!document.contains(SECRET), "secret escaped: {document}");
    assert!(!document.contains(FIRST), "first path escaped: {document}");
    assert!(
        !document.contains(SECOND),
        "second path escaped: {document}"
    );
}
