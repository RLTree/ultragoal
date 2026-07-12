use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityState, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const REGISTRY: &str = "migration/generated-surface-authority.json";

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn registry(surfaces: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": "GeneratedSurfaceAuthority-v2",
        "contract_id": CONTRACT_ID,
        "surfaces": surfaces
    }))
    .unwrap()
}

fn retained(output: &str, digest: &str) -> Value {
    json!({
        "disposition": "retained_context",
        "output": output,
        "sha256": digest,
        "reason": "Preserved only as bound migration context.",
        "replacement_targets": ["HCT-CLAIMS", "HCT-INVENTORY"],
        "preserve": true,
        "physical_deletion_authorized": false
    })
}

fn rejects(label: &str, bytes: &[u8]) {
    let repo = TestRepo::new(label);
    repo.write(REGISTRY, bytes);
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(
        error.to_string().contains("invalid registry"),
        "unexpected error: {error}"
    );
}

#[test]
fn retained_context_is_digest_bound_context_without_generator_authority() {
    let repo = TestRepo::new("generated-retained-context");
    let bytes = b"retained historical output\n";
    let output = "examples/generated/retained.json";
    repo.write(output, bytes);
    repo.write(REGISTRY, &registry(vec![retained(output, &sha256(bytes))]));
    repo.commit();

    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    let entry = catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == format!("GENERATED:{output}"))
        .unwrap();
    assert_eq!(entry.authority_state, AuthorityState::Context);
    assert_eq!(entry.active_status, ActiveStatus::ContextOnly);
    assert_eq!(entry.digest_sha256, sha256(bytes));
    assert_eq!(entry.generator, None);
    assert_eq!(entry.input_provenance, [REGISTRY]);
    assert_eq!(entry.references, ["HCT-CLAIMS", "HCT-INVENTORY"]);
    assert!(!catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(output)
            && finding.code == "generated_surface_missing_provenance"
    }));
}

#[test]
fn generated_authority_v2_rejects_mixed_unknown_duplicate_and_unsafe_rows() {
    let digest = "a".repeat(64);
    let canonical = json!({
        "disposition":"canonical_projection",
        "output":"generated/shared.json",
        "generator":"HCT-INVENTORY",
        "recipe":"input-digest-index-v1",
        "inputs":["source.txt"]
    });
    let invalid = vec![
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/mixed.json",
            "sha256":digest, "reason":"context", "replacement_targets":["HCT-CLAIMS"],
            "preserve":true, "physical_deletion_authorized":false,
            "generator":"HCT-INVENTORY"
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/upper.json",
            "sha256":"A".repeat(64), "reason":"context",
            "replacement_targets":["HCT-CLAIMS"], "preserve":true,
            "physical_deletion_authorized":false
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/extra.json",
            "sha256":"b".repeat(64), "reason":"context",
            "replacement_targets":["HCT-CLAIMS"], "preserve":true,
            "physical_deletion_authorized":false, "extra":true
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/preserve.json",
            "sha256":"c".repeat(64), "reason":"context",
            "replacement_targets":["HCT-CLAIMS"], "preserve":false,
            "physical_deletion_authorized":false
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/delete.json",
            "sha256":"d".repeat(64), "reason":"context",
            "replacement_targets":["HCT-CLAIMS"], "preserve":true,
            "physical_deletion_authorized":true
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/order.json",
            "sha256":"e".repeat(64), "reason":"context",
            "replacement_targets":["HCT-INVENTORY", "HCT-CLAIMS"], "preserve":true,
            "physical_deletion_authorized":false
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/traversal.json",
            "sha256":"e".repeat(64), "reason":"context",
            "replacement_targets":["../../secret"], "preserve":true,
            "physical_deletion_authorized":false
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/plain.json",
            "sha256":"e".repeat(64), "reason":"context",
            "replacement_targets":["foo"], "preserve":true,
            "physical_deletion_authorized":false
        })]),
        registry(vec![json!({
            "disposition":"canonical_projection", "output":"generated//x.json",
            "generator":"HCT-INVENTORY", "recipe":"input-digest-index-v1",
            "inputs":["source.txt"]
        })]),
        registry(vec![json!({
            "disposition":"retained_context", "output":"generated/",
            "sha256":"e".repeat(64), "reason":"context",
            "replacement_targets":["HCT-CLAIMS"], "preserve":true,
            "physical_deletion_authorized":false
        })]),
        registry(vec![canonical.clone(), retained("generated/shared.json", &digest)]),
        registry(vec![json!({
            "disposition":"retained_context", "sha256":"f".repeat(64),
            "reason":"context", "replacement_targets":["HCT-CLAIMS"],
            "preserve":true, "physical_deletion_authorized":false
        })]),
        format!(
            "{{\"schema_version\":\"GeneratedSurfaceAuthority-v2\",\"contract_id\":\"{CONTRACT_ID}\",\"surfaces\":[{{\"disposition\":\"retained_context\",\"output\":\"generated/duplicate.json\",\"sha256\":\"{digest}\",\"sha256\":\"{digest}\",\"reason\":\"context\",\"replacement_targets\":[\"HCT-CLAIMS\"],\"preserve\":true,\"physical_deletion_authorized\":false}}]}}"
        )
        .into_bytes(),
    ];
    for (index, bytes) in invalid.iter().enumerate() {
        rejects(&format!("generated-invalid-v2-{index}"), bytes);
    }
}

#[test]
fn generated_authority_v2_keeps_registry_and_collection_bounds() {
    let surfaces = (0..257)
        .map(|index| {
            json!({
                "disposition":"canonical_projection",
                "output":format!("generated/{index:03}.json"),
                "generator":"HCT-INVENTORY",
                "recipe":"input-digest-index-v1",
                "inputs":["source.txt"]
            })
        })
        .collect();
    rejects("generated-surface-count", &registry(surfaces));

    let inputs = (0..257)
        .map(|index| format!("inputs/{index:03}.txt"))
        .collect::<Vec<_>>();
    rejects(
        "generated-input-count",
        &registry(vec![json!({
            "disposition":"canonical_projection", "output":"generated/many-inputs.json",
            "generator":"HCT-INVENTORY", "recipe":"input-digest-index-v1", "inputs":inputs
        })]),
    );

    let replacements = (0..257)
        .map(|index| format!("HCT-X{index:03}"))
        .collect::<Vec<_>>();
    rejects(
        "generated-replacement-count",
        &registry(vec![json!({
            "disposition":"retained_context", "output":"generated/many-targets.json",
            "sha256":"a".repeat(64), "reason":"context",
            "replacement_targets":replacements, "preserve":true,
            "physical_deletion_authorized":false
        })]),
    );

    let oversized = TestRepo::new("generated-registry-bytes");
    let mut bytes = registry(Vec::new());
    bytes.extend(std::iter::repeat_n(b' ', 1024 * 1024));
    oversized.write(REGISTRY, &bytes);
    oversized.commit();
    let context = LiveContext::build(inventory_request(&oversized.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("byte limit"));
}

#[test]
fn retained_context_missing_and_tamper_are_causal_findings() {
    let missing = TestRepo::new("generated-retained-missing");
    let missing_output = "generated/missing-context.json";
    missing.write(
        REGISTRY,
        &registry(vec![retained(missing_output, &"a".repeat(64))]),
    );
    missing.commit();
    let context = LiveContext::build(inventory_request(&missing.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(missing_output)
            && finding.code == "retained_context_output_missing"
    }));

    let tampered = TestRepo::new("generated-retained-tampered");
    let tampered_output = "generated/tampered-context.json";
    tampered.write(tampered_output, b"actual bytes");
    tampered.write(
        REGISTRY,
        &registry(vec![retained(tampered_output, &"b".repeat(64))]),
    );
    tampered.commit();
    let context = LiveContext::build(inventory_request(&tampered.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(tampered_output)
            && finding.code == "retained_context_digest_mismatch"
    }));
    let entry = catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == format!("GENERATED:{tampered_output}"))
        .expect("tampered physical surface remains inventoried");
    assert_eq!(entry.authority_state, AuthorityState::Legacy);
    assert_eq!(entry.active_status, ActiveStatus::Active);
    assert_eq!(entry.generator, None);
    assert!(entry.references.is_empty());
}

#[cfg(unix)]
#[test]
fn retained_context_symlink_and_special_outputs_are_rejected() {
    let linked = TestRepo::new("generated-retained-symlink");
    linked.write("target.bin", b"target");
    let linked_output = "generated/linked-context.json";
    fs::create_dir_all(linked.root.join("generated")).unwrap();
    std::os::unix::fs::symlink("../target.bin", linked.root.join(linked_output)).unwrap();
    linked.write(
        REGISTRY,
        &registry(vec![retained(linked_output, &sha256(b"target"))]),
    );
    linked.commit();
    let context = LiveContext::build(inventory_request(&linked.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("regular non-symlink file"));

    let special = TestRepo::new("generated-retained-special");
    let special_output = "generated/special-context.json";
    fs::create_dir_all(special.root.join("generated")).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(special.root.join(special_output))
            .status()
            .unwrap()
            .success()
    );
    special.write(
        REGISTRY,
        &registry(vec![retained(special_output, &"c".repeat(64))]),
    );
    special.commit();
    let context = LiveContext::build(inventory_request(&special.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.relative_path.as_deref() == Some(special_output)
            && finding.code == "retained_context_output_not_regular"
    }));
}
