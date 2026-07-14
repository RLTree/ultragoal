#![cfg(unix)]

use super::{Catalog, REGISTRY_PATH};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const OUTPUT: &str = "docs/generated/observability/command-inventory.json";
pub(super) const INPUT: &str = "inputs/source.txt";
pub(super) const SECRET: &str = "SECRET_CANARY must never be read or echoed";

pub(super) fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("generated directory");
    fs::create_dir_all(root.join("inputs")).expect("input directory");
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

pub(super) fn digest(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string()
}

pub(super) fn retained(bytes: &[u8]) -> serde_json::Value {
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

pub(super) fn source_projection(root: &Path) -> serde_json::Value {
    let output = fs::read(root.join(OUTPUT)).expect("projection output");
    json!({
        "disposition": "source_projection",
        "output": OUTPUT,
        "generator": "scripts/project-agent-standards",
        "canonical_sources": [INPUT],
        "regeneration_command": "scripts/project-agent-standards write",
        "output_sha256": digest(&output)
    })
}

pub(super) fn write_registry(root: &Path, surface: serde_json::Value) {
    fs::write(
        root.join(REGISTRY_PATH),
        serde_json::to_vec(&json!({
            "schema_version": "GeneratedSurfaceAuthority-v3",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "registry_projection": {
                "generator": "scripts/project-generated-authority",
                "canonical_sources": ["migration/generated-surface-authority/test-shard.json"],
                "regeneration_command": "scripts/project-generated-authority write"
            },
            "surfaces": [surface]
        }))
        .expect("registry bytes"),
    )
    .expect("registry");
}

pub(super) fn write_manifest(root: &Path, paths: &[&str]) {
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": paths})).expect("manifest bytes"),
    )
    .expect("manifest");
}

pub(super) fn assert_rejected_without_secret(error: &str) {
    assert!(error.contains("anchored package"), "{error}");
    assert!(
        !error.contains(SECRET),
        "secret escaped through error: {error}"
    );
}

mod descriptor_identity;
mod directory_substitution;
mod session_revalidation;
