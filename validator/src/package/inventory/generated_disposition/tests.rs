use super::{Catalog, Classification, REGISTRY_PATH};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT: &str = "docs/generated/observability/command-inventory.json";

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("generated directory");
    fs::create_dir_all(root.join("migration/generated-surface-authority"))
        .expect("migration directory");
    fs::create_dir_all(root.join("scripts")).expect("scripts directory");
    fs::write(
        root.join("scripts/project-generated-authority"),
        b"generator",
    )
    .expect("registry generator");
    fs::write(root.join("scripts/project-agent-standards"), b"generator")
        .expect("projection generator");
    fs::write(
        root.join("migration/generated-surface-authority/test-shard.json"),
        b"{}",
    )
    .expect("registry source");
    root
}

fn digest(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string()
}

fn retained(output: &str, bytes: &[u8]) -> serde_json::Value {
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

fn write_registry(root: &Path, surfaces: serde_json::Value) -> Vec<u8> {
    let bytes = serde_json::to_vec(&json!({
        "schema_version": "GeneratedSurfaceAuthority-v3",
        "contract_id": "harness-ultragoal-successor-contract-v2",
        "registry_projection": {
            "generator": "scripts/project-generated-authority",
            "canonical_sources": ["migration/generated-surface-authority/test-shard.json"],
            "regeneration_command": "scripts/project-generated-authority write"
        },
        "surfaces": surfaces
    }))
    .expect("registry bytes");
    fs::write(root.join(REGISTRY_PATH), &bytes).expect("registry");
    bytes
}

fn write_manifest(root: &Path, paths: &[&str]) {
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": paths})).expect("manifest bytes"),
    )
    .expect("manifest");
}

#[test]
fn retained_context_is_verified_and_omitted_from_package_identity() {
    let root = root("package-retained-generated-context");
    let bytes = b"SECRET_CANARY retained predecessor";
    fs::write(root.join(OUTPUT), bytes).expect("retained output");
    let registry = write_registry(&root, json!([retained(OUTPUT, bytes)]));
    write_manifest(&root, &[OUTPUT]);

    let actual = super::super::package_digest(&root).expect("package identity");
    let mut payload = Vec::new();
    payload.extend_from_slice(REGISTRY_PATH.as_bytes());
    payload.push(0);
    payload.extend_from_slice(&registry);
    payload.push(0);
    assert_eq!(actual, crate::digest::bytes(&payload));

    let catalog = Catalog::load(&root).expect("catalog");
    assert_eq!(
        catalog.classify(&root, OUTPUT).expect("retained row"),
        Classification::RetainedContext {
            replacement_targets: vec!["HCT-OBSERVE".to_string()]
        }
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn retained_context_rejects_missing_tampered_and_unregistered_outputs() {
    let root = root("package-retained-generated-failures");
    let bytes = b"retained";
    fs::write(root.join(OUTPUT), bytes).expect("retained output");
    write_manifest(&root, &[OUTPUT]);
    let other = "docs/generated/observability/other.json";
    fs::write(root.join(other), b"other").expect("other output");
    write_registry(&root, json!([retained(other, b"other")]));
    assert!(
        super::super::package_digest(&root)
            .expect_err("missing row")
            .contains("no adopted disposition")
    );

    write_registry(&root, json!([retained(OUTPUT, bytes)]));
    fs::write(root.join(OUTPUT), b"tampered").expect("tamper");
    assert!(
        super::super::package_digest(&root)
            .expect_err("tampered output")
            .contains("digest mismatch")
    );
    fs::remove_file(root.join(OUTPUT)).expect("remove retained output");
    assert!(
        super::super::package_digest(&root)
            .expect_err("missing output")
            .contains("unavailable")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn retained_context_rejects_symlinks_and_fifos_without_reading_them() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let root = root("package-retained-generated-special-files");
    let output = root.join(OUTPUT);
    let outside = root.join("outside-secret");
    fs::write(&outside, b"SECRET_CANARY").expect("outside");
    write_registry(&root, json!([retained(OUTPUT, b"SECRET_CANARY")]));
    write_manifest(&root, &[OUTPUT]);
    std::os::unix::fs::symlink(&outside, &output).expect("symlink");
    assert!(super::super::package_digest(&root).is_err());

    fs::remove_file(&output).expect("remove symlink");
    let fifo = CString::new(output.as_os_str().as_bytes()).expect("fifo path");
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(super::super::package_digest(&root).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn source_projection_is_verified_but_cannot_enter_package_identity() {
    let root = root("package-canonical-generated-output");
    let input = "docs/input.txt";
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join(input), b"input").expect("input");
    let input_row = json!({"path": input, "sha256": digest(b"input")});
    let output = serde_json::to_vec(&json!({
        "_meta": {
            "generator": "HCT-INVENTORY",
            "inputs": [input_row.clone()],
            "recipe": "input-digest-index-v1"
        },
        "entries": [input_row]
    }))
    .expect("output");
    fs::write(root.join(OUTPUT), &output).expect("output");
    write_registry(
        &root,
        json!([{
            "disposition": "source_projection",
            "output": OUTPUT,
            "generator": "scripts/project-agent-standards",
            "canonical_sources": [input],
            "regeneration_command": "scripts/project-agent-standards write",
            "output_sha256": digest(&output)
        }]),
    );
    write_manifest(&root, &[input, OUTPUT]);
    assert!(
        super::super::package_digest(&root)
            .expect_err("provenance-only output")
            .contains("provenance-only")
    );
    fs::write(root.join(OUTPUT), b"drift").expect("drift");
    assert!(
        super::super::package_digest(&root)
            .expect_err("projection drift")
            .contains("digest mismatch")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn malformed_or_duplicate_key_registry_fails_closed() {
    let root = root("package-generated-registry-malformed");
    fs::write(root.join(OUTPUT), b"retained").expect("output");
    write_manifest(&root, &[OUTPUT]);
    fs::write(root.join(REGISTRY_PATH), b"{").expect("malformed");
    assert!(super::super::package_digest(&root).is_err());
    fs::write(
        root.join(REGISTRY_PATH),
        br#"{"schema_version":"GeneratedSurfaceAuthority-v3","schema_version":"GeneratedSurfaceAuthority-v3","contract_id":"harness-ultragoal-successor-contract-v2","registry_projection":{"generator":"scripts/project-generated-authority","canonical_sources":["migration/generated-surface-authority/test-shard.json"],"regeneration_command":"scripts/project-generated-authority write"},"surfaces":[]}"#,
    )
    .expect("duplicate key registry");
    assert!(super::super::package_digest(&root).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn duplicate_manifest_paths_fail_closed_before_identity_projection() {
    let root = root("package-generated-duplicate-manifest-path");
    let input = "docs/input.txt";
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join(input), b"input").expect("input");
    write_manifest(&root, &[input, input]);
    let error = super::super::package_digest(&root).expect_err("duplicate manifest path");
    assert!(error.contains("duplicate inventory path"), "{error}");
    fs::remove_dir_all(root).expect("cleanup");
}
