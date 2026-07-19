use super::{Catalog, Classification, REGISTRY_PATH};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";
const SOURCE: &str = "GOAL_CONTRACT.md";
const SCHEMA: &str = "schemas/product-success-contract.schema.json";
const AMENDMENTS: &str = "AMENDMENTS.jsonl";
const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const AMENDMENT_HASH: &str = "ea134939717ab2422a444f40eed9ca6b388e2d86846a74ce2f0aff823cb95600";
const OUTPUT_BYTES: &[u8] =
    include_bytes!("../../../../../examples/generated/PRODUCT_SUCCESS_CONTRACT.json");
const SOURCE_BYTES: &[u8] = include_bytes!("../../../../../GOAL_CONTRACT.md");
const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../../../schemas/product-success-contract.schema.json");
const AMENDMENT_BYTES: &[u8] = include_bytes!("../../../../../AMENDMENTS.jsonl");

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
    fs::write(root.join(SOURCE), SOURCE_BYTES).expect("goal");
    fs::write(root.join(SCHEMA), SCHEMA_BYTES).expect("schema");
    fs::write(root.join(AMENDMENTS), AMENDMENT_BYTES).expect("amendments");
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

fn write_contract(root: &Path, receipt: &str) {
    let source_digest = digest(SOURCE_BYTES);
    let schema_digest = digest(SCHEMA_BYTES);
    let output = if receipt == ZERO {
        OUTPUT_BYTES.to_vec()
    } else {
        let mut contract: serde_json::Value =
            serde_json::from_slice(OUTPUT_BYTES).expect("contract");
        contract["receipt_digest"] = json!(format!("sha256:{receipt}"));
        let mut bytes = serde_json::to_vec_pretty(&contract).expect("contract");
        bytes.push(b'\n');
        bytes
    };
    fs::write(root.join(OUTPUT), &output).expect("contract output");
    let output_digest = digest(&output);
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
            "schema_sha256": schema_digest,
            "source_contract": SOURCE,
            "source_contract_sha256": source_digest,
            "amendment_log": AMENDMENTS,
            "amendment_id": "AMEND-003",
            "amendment_hash": AMENDMENT_HASH,
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
    write_contract(&root, ZERO);
    assert!(super::super::package_digest(&root).is_ok());
    let catalog = Catalog::load(&root).expect("catalog");
    assert_eq!(
        catalog.classify(&root, OUTPUT).expect("classification"),
        Classification::AdoptedSchemaContract
    );

    write_contract(&root, &"1".repeat(64));
    assert!(
        super::super::package_digest(&root)
            .expect_err("claim-bearing receipt")
            .contains("authority fields invalid")
    );

    write_contract(&root, ZERO);
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
            .contains("amendment invalid")
    );

    write_contract(&root, ZERO);
    fs::write(root.join(SCHEMA), b"{\"type\":\"null\"}").expect("schema substitution");
    assert!(
        super::super::package_digest(&root)
            .expect_err("schema substitution")
            .contains("schema digest mismatch")
    );
    fs::remove_dir_all(root).expect("cleanup");
}
