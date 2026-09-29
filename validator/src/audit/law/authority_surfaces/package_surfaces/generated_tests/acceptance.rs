use super::super::State;
use super::fixture::{FIRST, SECOND, file_tree, inventory, retained_fixture};
use std::fs;

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
