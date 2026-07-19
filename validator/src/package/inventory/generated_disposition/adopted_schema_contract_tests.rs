use super::{Catalog, Classification, REGISTRY_PATH};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";
const SOURCE: &str = "GOAL_CONTRACT.md";
const SCHEMA: &str = "schemas/product-success-contract.schema.json";
const AMENDMENTS: &str = "AMENDMENTS.jsonl";
const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    for directory in [
        "examples/generated",
        "migration/generated-surface-authority",
        "schemas",
        "scripts",
    ] {
        fs::create_dir_all(root.join(directory)).expect("fixture directory");
    }
    fs::write(
        root.join("scripts/project-generated-authority"),
        b"generator",
    )
    .expect("generator");
    fs::write(
        root.join("migration/generated-surface-authority/product.json"),
        b"{}",
    )
    .expect("shard");
    fs::write(root.join(SOURCE), b"goal contract").expect("goal");
    fs::write(root.join(SCHEMA), b"{}").expect("schema");
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": [AMENDMENTS, SOURCE, OUTPUT, SCHEMA]}))
            .expect("manifest"),
    )
    .expect("manifest");
    root
}

fn digest(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_owned()
}

fn write_contract(root: &Path, receipt: &str, registry_hash: &str) {
    let source_digest = digest(b"goal contract");
    let output = serde_json::to_vec(&json!({
        "schema": "harness-ultragoal.product-success-contract.v1",
        "target_revision": {"path": SOURCE, "digest": format!("sha256:{source_digest}")},
        "receipt_digest": format!("sha256:{receipt}")
    }))
    .expect("contract");
    fs::write(root.join(OUTPUT), &output).expect("contract output");
    let output_digest = digest(&output);
    let amendment_hash = format!("sha256:{registry_hash}");
    let amendment = json!({
        "amendment_id": "AMEND-003",
        "amendment_hash": amendment_hash,
        "new_contract_hash": format!("sha256:{source_digest}"),
        "backlog_updates": [{"path": OUTPUT, "digest": format!("sha256:{output_digest}")}]
    });
    fs::write(
        root.join(AMENDMENTS),
        format!(
            "{}\n",
            serde_json::to_string(&amendment).expect("amendment")
        ),
    )
    .expect("amendments");
    let registry = json!({
        "schema_version": "GeneratedSurfaceAuthority-v3",
        "contract_id": "harness-ultragoal-successor-contract-v2",
        "registry_projection": {
            "generator": "scripts/project-generated-authority",
            "canonical_sources": ["migration/generated-surface-authority/product.json"],
            "regeneration_command": "scripts/project-generated-authority write"
        },
        "surfaces": [{
            "disposition": "adopted_schema_contract",
            "output": OUTPUT,
            "sha256": output_digest,
            "schema": SCHEMA,
            "source_contract": SOURCE,
            "source_contract_sha256": source_digest,
            "amendment_log": AMENDMENTS,
            "amendment_id": "AMEND-003",
            "amendment_hash": registry_hash,
            "claim_ceiling": "contract_authority_only"
        }]
    });
    fs::write(
        root.join(REGISTRY_PATH),
        serde_json::to_vec(&registry).expect("registry"),
    )
    .expect("registry");
}

#[test]
fn adopted_contract_enters_package_identity_without_raising_claims() {
    let root = root("package-adopted-schema-contract");
    write_contract(&root, ZERO, ZERO);
    assert!(super::super::package_digest(&root).is_ok());
    let catalog = Catalog::load(&root).expect("catalog");
    assert_eq!(
        catalog.classify(&root, OUTPUT).expect("classification"),
        Classification::AdoptedSchemaContract
    );

    write_contract(&root, &"1".repeat(64), ZERO);
    assert!(
        super::super::package_digest(&root)
            .expect_err("claim-bearing receipt")
            .contains("authority fields invalid")
    );

    write_contract(&root, ZERO, ZERO);
    let mut registry: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(REGISTRY_PATH)).expect("registry"))
            .expect("registry JSON");
    registry["surfaces"][0]["amendment_hash"] = json!("1".repeat(64));
    fs::write(
        root.join(REGISTRY_PATH),
        serde_json::to_vec(&registry).expect("registry"),
    )
    .expect("registry");
    assert!(
        super::super::package_digest(&root)
            .expect_err("amendment substitution")
            .contains("amendment binding invalid")
    );
    fs::remove_dir_all(root).expect("cleanup");
}
