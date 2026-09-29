use super::fixture::{
    FIRST, INPUT, REGISTRY, SECOND, SECRET, assert_whole_batch_invalid, canonical_fixture,
    retained_fixture,
};
use crate::package::inventory::anchored::test_hooks;
use std::fs;
use std::io::Write;

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
