#![cfg(unix)]

use super::State;
use crate::package::inventory::anchored::test_hooks;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const FIRST: &str = "docs/generated/observability/command-inventory.json";
const SECOND: &str = "examples/generated/review-debrief.json";
const INPUT: &str = "inputs/source.txt";
const REGISTRY: &str = "migration/generated-surface-authority.json";
const SECRET: &str = "SECRET_CANARY must never escape through an error";

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

fn retained_fixture(label: &str) -> PathBuf {
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

fn canonical_fixture(label: &str) -> PathBuf {
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

fn inventory() -> BTreeSet<String> {
    [FIRST.to_string(), SECOND.to_string()]
        .into_iter()
        .collect()
}

fn file_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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

fn assert_whole_batch_invalid(root: &Path) {
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
        "one fixed batch failure must cover every requested path: {failures:?}"
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

#[test]
fn batch_classification_succeeds_only_after_complete_snapshot_validation() {
    let root = retained_fixture("generated-batch-stable");
    let state = State::build(&root, &inventory());
    for path in [FIRST, SECOND] {
        assert_eq!(state.row(path).authority_level, "retained_context_no_claim");
    }
    assert!(state.failures().is_empty());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn batch_classification_has_zero_hidden_writes() {
    let root = retained_fixture("generated-batch-read-only");
    let before = file_tree(&root);
    let state = State::build(&root, &inventory());
    assert!(state.failures().is_empty());
    assert_eq!(file_tree(&root), before);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn one_failed_classification_invalidates_every_row_in_the_batch() {
    let root = retained_fixture("generated-batch-one-invalid-row");
    fs::write(root.join(SECOND), SECRET).expect("tamper second output");
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn registry_mutation_between_classifications_invalidates_whole_batch() {
    let root = retained_fixture("generated-batch-registry-mutation");
    let registry = root.join(REGISTRY);
    test_hooks::set_after_read(FIRST, move || {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&registry)
            .expect("open registry");
        file.write_all(b"\n").expect("mutate registry");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn input_mutation_between_classifications_invalidates_whole_batch() {
    let root = canonical_fixture("generated-batch-input-mutation");
    let input = root.join(INPUT);
    test_hooks::set_after_read(FIRST, move || {
        fs::write(input, format!("mutated {SECRET}")).expect("mutate input");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn output_mutation_between_classifications_invalidates_whole_batch() {
    let root = retained_fixture("generated-batch-output-mutation");
    let first = root.join(FIRST);
    test_hooks::set_after_read(FIRST, move || {
        fs::write(first, format!("mutated {SECRET}")).expect("mutate output");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn root_replacement_between_classifications_invalidates_whole_batch() {
    let root = retained_fixture("generated-batch-root-replacement");
    let held = root.with_extension("held");
    let replacement = root.with_extension("replacement");
    fs::create_dir(&replacement).expect("replacement root");
    let root_for_hook = root.clone();
    let held_for_hook = held.clone();
    test_hooks::set_after_read(FIRST, move || {
        fs::rename(&root_for_hook, &held_for_hook).expect("hold root");
        fs::rename(&replacement, &root_for_hook).expect("replace root");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(&root).expect("cleanup replacement");
    fs::remove_dir_all(held).expect("cleanup held root");
}

#[test]
fn ancestor_replacement_between_classifications_invalidates_whole_batch() {
    let root = retained_fixture("generated-batch-ancestor-replacement");
    let docs = root.join("docs");
    let held = root.join("held-docs");
    let replacement = root.join("replacement-docs");
    fs::create_dir(&replacement).expect("replacement docs");
    let root_for_hook = root.clone();
    test_hooks::set_after_read(FIRST, move || {
        fs::rename(docs, held).expect("hold docs");
        fs::rename(replacement, root_for_hook.join("docs")).expect("replace docs");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn leaf_replacement_between_classifications_invalidates_whole_batch() {
    let root = retained_fixture("generated-batch-leaf-replacement");
    let first = root.join(FIRST);
    let held = root.join("held-first.json");
    let replacement = root.join("replacement-first.json");
    fs::write(&replacement, SECRET).expect("replacement output");
    let root_for_hook = root.clone();
    test_hooks::set_after_read(FIRST, move || {
        fs::rename(first, held).expect("hold output");
        fs::rename(replacement, root_for_hook.join(FIRST)).expect("replace output");
    });
    assert_whole_batch_invalid(&root);
    fs::remove_dir_all(root).expect("cleanup");
}
