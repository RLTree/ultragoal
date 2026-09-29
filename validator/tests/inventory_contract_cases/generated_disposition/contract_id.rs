const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const REGISTRY: &str = "migration/generated-surface-authority.json";

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn registry(surfaces: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": "GeneratedSurfaceAuthority-v3",
        "contract_id": CONTRACT_ID,
        "registry_projection": {
            "generator": REGISTRY,
            "canonical_sources": [REGISTRY],
            "regeneration_command": format!("{REGISTRY} write")
        },
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

fn retained_target(output: &str, digest: &str, target: &str) -> Value {
    json!({
        "disposition": "retained_context",
        "output": output,
        "sha256": digest,
        "reason": "Preserved only as bound migration context.",
        "replacement_targets": [target],
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
fn generated_authority_v3_rejects_mixed_unknown_duplicate_and_unsafe_rows() {
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
        registry(vec![retained_target(
            "generated/empty-hct.json",
            &digest,
            "HCT-",
        )]),
        registry(vec![retained_target(
            "generated/empty-ps.json",
            &digest,
            "PS-",
        )]),
        registry(vec![retained_target(
            "generated/lower-hct.json",
            &digest,
            "HCT-lower",
        )]),
        registry(vec![retained_target(
            "generated/hyphen-ps.json",
            &digest,
            "PS--X",
        )]),
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
            "{{\"schema_version\":\"GeneratedSurfaceAuthority-v3\",\"contract_id\":\"{CONTRACT_ID}\",\"registry_projection\":{{\"generator\":\"{REGISTRY}\",\"canonical_sources\":[\"{REGISTRY}\"],\"regeneration_command\":\"{REGISTRY} write\"}},\"surfaces\":[{{\"disposition\":\"retained_context\",\"output\":\"generated/duplicate.json\",\"sha256\":\"{digest}\",\"sha256\":\"{digest}\",\"reason\":\"context\",\"replacement_targets\":[\"HCT-CLAIMS\"],\"preserve\":true,\"physical_deletion_authorized\":false}}]}}"
        )
        .into_bytes(),
    ];
    for (index, bytes) in invalid.iter().enumerate() {
        rejects(&format!("generated-invalid-v3-{index}"), bytes);
    }
}
