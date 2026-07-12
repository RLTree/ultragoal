#![cfg(unix)]

use super::{Catalog, REGISTRY_PATH, anchored};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT: &str = "docs/generated/observability/command-inventory.json";
const INPUT: &str = "inputs/source.txt";
const SECRET: &str = "SECRET_CANARY must never be read or echoed";

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("generated directory");
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

fn retained(bytes: &[u8]) -> serde_json::Value {
    json!({
        "disposition": "retained_context",
        "output": OUTPUT,
        "sha256": digest(bytes),
        "reason": "preserved predecessor context",
        "replacement_targets": ["HCT-OBSERVE"],
        "preserve": true,
        "physical_deletion_authorized": false
    })
}

fn canonical() -> serde_json::Value {
    json!({
        "disposition": "canonical_projection",
        "output": OUTPUT,
        "generator": "HCT-INVENTORY",
        "recipe": "input-digest-index-v1",
        "inputs": [INPUT]
    })
}

fn write_registry(root: &Path, surface: serde_json::Value) {
    fs::write(
        root.join(REGISTRY_PATH),
        serde_json::to_vec(&json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [surface]
        }))
        .expect("registry bytes"),
    )
    .expect("registry");
}

fn write_manifest(root: &Path, paths: &[&str]) {
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": paths})).expect("manifest bytes"),
    )
    .expect("manifest");
}

fn assert_rejected_without_secret(error: &str) {
    assert!(error.contains("anchored package"), "{error}");
    assert!(
        !error.contains(SECRET),
        "secret escaped through error: {error}"
    );
}

#[test]
fn descriptor_reads_reject_hardlinked_registry_input_and_output() {
    let registry_root = root("generated-registry-hardlink");
    fs::write(registry_root.join(OUTPUT), b"retained").expect("output");
    write_registry(&registry_root, retained(b"retained"));
    write_manifest(&registry_root, &[OUTPUT]);
    fs::hard_link(
        registry_root.join(REGISTRY_PATH),
        registry_root.join("registry-hardlink.json"),
    )
    .expect("registry hardlink");
    let error = super::super::package_digest(&registry_root).expect_err("hardlinked registry");
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
    let error = super::super::package_digest(&output_root).expect_err("hardlinked output");
    assert!(error.contains("multiple hard links"), "{error}");
    fs::remove_dir_all(output_root).expect("cleanup output fixture");

    let input_root = root("generated-input-hardlink");
    fs::write(input_root.join(INPUT), b"input").expect("input");
    fs::write(input_root.join(OUTPUT), b"unused").expect("output");
    write_registry(&input_root, canonical());
    write_manifest(&input_root, &[INPUT, OUTPUT]);
    fs::hard_link(
        input_root.join(INPUT),
        input_root.join("input-hardlink.txt"),
    )
    .expect("input hardlink");
    let error = super::super::package_digest(&input_root).expect_err("hardlinked input");
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
    fs::write(outside.join("generated-surface-authority.json"), SECRET).expect("secret");
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
        SECRET,
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
    write_registry(&root, canonical());
    let catalog = Catalog::load(&root).expect("catalog");
    let outside = root.join("outside-inputs");
    fs::create_dir(&outside).expect("outside inputs");
    fs::write(outside.join("source.txt"), SECRET).expect("secret");
    let inputs = root.join("inputs");
    let owned = root.join("inputs-owned");
    anchored::set_before_component_open(move || {
        fs::rename(&inputs, &owned).expect("move inputs");
        std::os::unix::fs::symlink(&outside, &inputs).expect("substitute inputs");
    });
    let error = catalog
        .classify(&root, OUTPUT)
        .err()
        .expect("substituted input ancestor");
    assert_rejected_without_secret(&error);
    fs::remove_dir_all(root).expect("cleanup");
}

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
    write_registry(&input_root, canonical());
    let catalog = Catalog::load(&input_root).expect("catalog");
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
        &catalog
            .classify(&input_root, OUTPUT)
            .expect_err("real input replacement"),
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

#[test]
fn shared_package_session_rejects_generated_cross_resource_mutation() {
    let root = root("generated-cross-resource-mutation");
    fs::write(root.join(INPUT), b"input").expect("input");
    let input_row = json!({"path": INPUT, "sha256": digest(b"input")});
    let output = serde_json::to_vec(&json!({
        "_meta": {"generator": "HCT-INVENTORY", "inputs": [input_row.clone()], "recipe": "input-digest-index-v1"},
        "entries": [input_row]
    }))
    .expect("output bytes");
    fs::write(root.join(OUTPUT), output).expect("output");
    write_registry(&root, canonical());
    write_manifest(&root, &[INPUT, OUTPUT]);
    let input = root.join(INPUT);
    super::super::anchored::test_hooks::set_before_component(OUTPUT, 0, move || {
        fs::write(&input, SECRET).expect("mutate generated input");
    });
    assert_rejected_without_secret(
        &super::super::package_digest(&root).expect_err("mixed generated snapshot"),
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn generated_disposition_errors_do_not_echo_untrusted_safe_paths() {
    const CANARY_PATH: &str = "docs/generated/SECRET_CANARY.json";
    let root = root("generated-untrusted-path-non-echo");
    fs::write(root.join(CANARY_PATH), b"context").expect("generated context");
    fs::write(
        root.join(REGISTRY_PATH),
        br#"{"schema_version":"GeneratedSurfaceAuthority-v2","contract_id":"harness-ultragoal-successor-contract-v2","surfaces":[]}"#,
    )
    .expect("empty registry");
    write_manifest(&root, &[CANARY_PATH]);
    let error = super::super::package_digest(&root).expect_err("missing disposition");
    assert!(error.contains("no adopted disposition"), "{error}");
    assert!(!error.contains("SECRET_CANARY"), "{error}");
    fs::remove_dir_all(root).expect("cleanup");
}
