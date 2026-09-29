use super::{
    Catalog, INPUT, OUTPUT, assert_rejected_without_secret, retained, root, source_projection,
    write_manifest, write_registry,
};
use crate::package::inventory::generated_disposition::anchored;
use std::fs;

#[test]
fn descriptor_reads_reject_hardlinked_registry_input_and_output() {
    let registry_root = root("generated-registry-hardlink");
    fs::write(registry_root.join(OUTPUT), b"retained").expect("output");
    write_registry(&registry_root, retained(b"retained"));
    write_manifest(&registry_root, &[OUTPUT]);
    fs::hard_link(
        registry_root.join(super::super::REGISTRY_PATH),
        registry_root.join("registry-hardlink.json"),
    )
    .expect("registry hardlink");
    let error =
        super::super::super::package_digest(&registry_root).expect_err("hardlinked registry");
    assert!(error.contains("multiple hard links"), "{error}");
    fs::remove_dir_all(registry_root).expect("cleanup registry fixture");

    let output_root = root("generated-output-hardlink");
    fs::write(output_root.join(OUTPUT), b"retained").expect("output");
    write_registry(&output_root, retained(b"retained"));
    write_manifest(&output_root, &[OUTPUT]);
    fs::hard_link(
        output_root.join(OUTPUT),
        output_root.join("output-hardlink.json"),
    )
    .expect("output hardlink");
    let error = super::super::super::package_digest(&output_root).expect_err("hardlinked output");
    assert!(error.contains("multiple hard links"), "{error}");
    fs::remove_dir_all(output_root).expect("cleanup output fixture");

    let input_root = root("generated-input-hardlink");
    fs::write(input_root.join(INPUT), b"input").expect("input");
    fs::write(input_root.join(OUTPUT), b"unused").expect("output");
    write_registry(&input_root, source_projection(&input_root));
    write_manifest(&input_root, &[INPUT, OUTPUT]);
    fs::hard_link(
        input_root.join(INPUT),
        input_root.join("input-hardlink.txt"),
    )
    .expect("input hardlink");
    let error = super::super::super::package_digest(&input_root).expect_err("hardlinked input");
    assert!(error.contains("multiple hard links"), "{error}");
    fs::remove_dir_all(input_root).expect("cleanup input fixture");
}

#[test]
fn descriptor_reads_reject_ancestor_symlink_substitution_for_registry() {
    let root = root("generated-registry-ancestor-substitution");
    fs::write(root.join(OUTPUT), b"retained").expect("output");
    write_registry(&root, retained(b"retained"));
    write_manifest(&root, &[OUTPUT]);
    let outside = root.join("outside-migration");
    fs::create_dir(&outside).expect("outside migration");
    fs::write(
        outside.join("generated-surface-authority.json"),
        super::SECRET,
    )
    .expect("secret");
    let migration = root.join("migration");
    let owned = root.join("migration-owned");
    anchored::set_before_component_open(move || {
        fs::rename(&migration, &owned).expect("move migration");
        std::os::unix::fs::symlink(&outside, &migration).expect("substitute migration");
    });
    let error = Catalog::load(&root)
        .err()
        .expect("substituted registry ancestor");
    assert_rejected_without_secret(&error);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn descriptor_reads_reject_ancestor_symlink_substitution_for_output() {
    let root = root("generated-output-ancestor-substitution");
    fs::write(root.join(OUTPUT), b"retained").expect("output");
    write_registry(&root, retained(b"retained"));
    let catalog = Catalog::load(&root).expect("catalog");
    let outside = root.join("outside-docs");
    fs::create_dir_all(outside.join("generated/observability")).expect("outside output tree");
    fs::write(
        outside.join("generated/observability/command-inventory.json"),
        super::SECRET,
    )
    .expect("secret");
    let docs = root.join("docs");
    let owned = root.join("docs-owned");
    anchored::set_before_component_open(move || {
        fs::rename(&docs, &owned).expect("move docs");
        std::os::unix::fs::symlink(&outside, &docs).expect("substitute docs");
    });
    let error = catalog
        .classify(&root, OUTPUT)
        .err()
        .expect("substituted output ancestor");
    assert_rejected_without_secret(&error);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn descriptor_reads_reject_ancestor_symlink_substitution_for_input() {
    let root = root("generated-input-ancestor-substitution");
    fs::write(root.join(INPUT), b"input").expect("input");
    fs::write(root.join(OUTPUT), b"unused").expect("output");
    write_registry(&root, source_projection(&root));
    let outside = root.join("outside-inputs");
    fs::create_dir(&outside).expect("outside inputs");
    fs::write(outside.join("source.txt"), super::SECRET).expect("secret");
    let inputs = root.join("inputs");
    let owned = root.join("inputs-owned");
    super::super::super::anchored::test_hooks::set_before_component(INPUT, 0, move || {
        fs::rename(&inputs, &owned).expect("move inputs");
        std::os::unix::fs::symlink(&outside, &inputs).expect("substitute inputs");
    });
    let error = Catalog::load(&root)
        .err()
        .expect("substituted input ancestor");
    assert_rejected_without_secret(&error);
    fs::remove_dir_all(root).expect("cleanup");
}
