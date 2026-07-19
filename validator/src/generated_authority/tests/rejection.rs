use super::registry_samples::{LOWER_DIGEST, SourceProjectionSample};
use super::rejection_code;

#[test]
fn rejects_unknown_fields_and_trailing_data() {
    let unknown = SourceProjectionSample {
        extra_surface_field: ",\"unknown\":true",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&unknown.bytes()),
        "generated_authority_json_invalid"
    );

    let trailing = SourceProjectionSample {
        trailing: "{}",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&trailing.bytes()),
        "generated_authority_trailing_data"
    );
}

#[test]
fn rejects_old_schema_and_noncanonical_paths() {
    let old = SourceProjectionSample {
        schema: "GeneratedSurfaceAuthority-v2",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&old.bytes()),
        "generated_authority_schema_unsupported"
    );

    let unsorted = SourceProjectionSample {
        sources: "\"z/input.json\",\"a/input.json\"",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&unsorted.bytes()),
        "generated_authority_paths_noncanonical"
    );
}

#[test]
fn rejects_invalid_digest_cycles_and_command_mismatch() {
    let uppercase_digest = LOWER_DIGEST.replace('0', "A");
    let invalid_digest = SourceProjectionSample {
        digest: &uppercase_digest,
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&invalid_digest.bytes()),
        "generated_authority_digest_invalid"
    );

    let cycle = SourceProjectionSample {
        output: "source/input.json",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&cycle.bytes()),
        "generated_authority_source_projection_cycle"
    );

    let command = SourceProjectionSample {
        command: "different-tool write",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&command.bytes()),
        "generated_authority_command_invalid"
    );
}

#[test]
fn rejects_adopted_contract_that_can_raise_claims() {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema_version": "GeneratedSurfaceAuthority-v3",
        "contract_id": "harness-ultragoal-successor-contract-v2",
        "registry_projection": {
            "generator": "scripts/project-generated-authority",
            "canonical_sources": ["migration/generated-surface-authority/product.json"],
            "regeneration_command": "scripts/project-generated-authority write"
        },
        "surfaces": [{
            "disposition": "adopted_schema_contract",
            "output": "examples/generated/product.json",
            "sha256": LOWER_DIGEST,
            "schema": "schemas/product.json",
            "source_contract": "GOAL_CONTRACT.md",
            "source_contract_sha256": LOWER_DIGEST,
            "amendment_log": "AMENDMENTS.jsonl",
            "amendment_id": "AMEND-003",
            "amendment_hash": LOWER_DIGEST,
            "claim_ceiling": "claim_authority"
        }]
    }))
    .unwrap();
    assert_eq!(
        rejection_code(&bytes),
        "generated_authority_adopted_schema_contract_invalid"
    );
}
