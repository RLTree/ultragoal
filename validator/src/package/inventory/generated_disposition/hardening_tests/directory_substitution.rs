use super::{
    Catalog, INPUT, OUTPUT, SECRET, assert_rejected_without_secret, retained, root,
    source_projection, write_registry,
};
use crate::package::inventory::generated_disposition::anchored;
use std::fs;

#[test]
fn descriptor_reads_reject_real_directory_substitution_for_generated_paths() {
    let registry_root = root("generated-registry-real-substitution");
    fs::write(registry_root.join(OUTPUT), b"retained").expect("output");
    write_registry(&registry_root, retained(b"retained"));
    let replacement = registry_root.join("replacement-migration");
    fs::create_dir(&replacement).expect("replacement migration");
    fs::write(replacement.join("generated-surface-authority.json"), SECRET).expect("secret");
    let migration = registry_root.join("migration");
    let held = registry_root.join("migration-held");
    anchored::set_before_component_open(move || {
        fs::rename(&migration, &held).expect("hold migration");
        fs::rename(&replacement, &migration).expect("replace migration");
    });
    assert_rejected_without_secret(
        &Catalog::load(&registry_root)
            .err()
            .expect("real registry replacement"),
    );
    fs::remove_dir_all(registry_root).expect("cleanup registry");

    let input_root = root("generated-input-real-substitution");
    fs::write(input_root.join(INPUT), b"input").expect("input");
    fs::write(input_root.join(OUTPUT), b"unused").expect("output");
    write_registry(&input_root, source_projection(&input_root));
    let replacement = input_root.join("replacement-inputs");
    fs::create_dir(&replacement).expect("replacement inputs");
    fs::write(replacement.join("source.txt"), SECRET).expect("secret");
    let inputs = input_root.join("inputs");
    let held = input_root.join("inputs-held");
    anchored::set_before_component_open(move || {
        fs::rename(&inputs, &held).expect("hold inputs");
        fs::rename(&replacement, &inputs).expect("replace inputs");
    });
    assert_rejected_without_secret(
        &Catalog::load(&input_root)
            .err()
            .expect("real input replacement"),
    );
    fs::remove_dir_all(input_root).expect("cleanup input");

    let output_root = root("generated-output-real-substitution");
    fs::write(output_root.join(OUTPUT), b"retained").expect("output");
    write_registry(&output_root, retained(b"retained"));
    let catalog = Catalog::load(&output_root).expect("catalog");
    let replacement = output_root.join("replacement-docs");
    fs::create_dir_all(replacement.join("generated/observability")).expect("replacement docs");
    fs::write(
        replacement.join("generated/observability/command-inventory.json"),
        SECRET,
    )
    .expect("secret");
    let docs = output_root.join("docs");
    let held = output_root.join("docs-held");
    anchored::set_before_component_open(move || {
        fs::rename(&docs, &held).expect("hold docs");
        fs::rename(&replacement, &docs).expect("replace docs");
    });
    assert_rejected_without_secret(
        &catalog
            .classify(&output_root, OUTPUT)
            .expect_err("real output replacement"),
    );
    fs::remove_dir_all(output_root).expect("cleanup output");
}
