use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};
use sha2::{Digest, Sha256};

fn generated_bytes(path: &str, digest: &str, entries: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "_meta": {
            "generator": "HCT-INVENTORY",
            "inputs": [{"path": path, "sha256": digest}],
            "recipe": "input-digest-index-v1"
        },
        "entries": entries
    }))
    .unwrap()
}

#[test]
fn deterministic_generated_index_regenerates_and_tamper_drifts() {
    let repo = TestRepo::new("generated-regeneration");
    let source = b"current source";
    let unrelated = b"unrelated source";
    let digest = format!("{:x}", Sha256::digest(source));
    let unrelated_digest = format!("{:x}", Sha256::digest(unrelated));
    repo.write("source.txt", source);
    repo.write("unrelated.txt", unrelated);
    repo.write(
        "migration/generated-surface-authority.json",
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [
                {"disposition":"canonical_projection","output":"generated/missing.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]},
                {"disposition":"canonical_projection","output":"generated/no-generator.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]},
                {"disposition":"canonical_projection","output":"generated/repointed.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]},
                {"disposition":"canonical_projection","output":"generated/tampered.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]},
                {"disposition":"canonical_projection","output":"generated/valid.json","generator":"HCT-INVENTORY","recipe":"input-digest-index-v1","inputs":["source.txt"]}
            ]
        }))
        .unwrap()
        .as_slice(),
    );
    repo.write(
        "generated/valid.json",
        &generated_bytes(
            "source.txt",
            &digest,
            serde_json::json!([{"path": "source.txt", "sha256": digest}]),
        ),
    );
    repo.write(
        "generated/tampered.json",
        &generated_bytes("source.txt", &digest, serde_json::json!([])),
    );
    repo.write(
        "generated/no-generator.json",
        &serde_json::to_vec(&serde_json::json!({
            "_meta": {
                "inputs": [{"path": "source.txt", "sha256": digest}],
                "recipe": "input-digest-index-v1"
            },
            "entries": [{"path": "source.txt", "sha256": digest}]
        }))
        .unwrap(),
    );
    repo.write(
        "generated/repointed.json",
        &generated_bytes(
            "unrelated.txt",
            &unrelated_digest,
            serde_json::json!([{"path": "unrelated.txt", "sha256": unrelated_digest}]),
        ),
    );
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(!catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some("generated/valid.json")
            && matches!(
                finding.code.as_str(),
                "generated_output_drift" | "generated_output_regeneration_required"
            )
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some("generated/tampered.json")
            && finding.code == "generated_output_drift"
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some("generated/no-generator.json")
            && finding.code == "invalid_generated_provenance"
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some("generated/repointed.json")
            && finding.code == "generated_output_drift"
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some("generated/missing.json")
            && finding.code == "registered_generated_surface_missing"
    }));
}
