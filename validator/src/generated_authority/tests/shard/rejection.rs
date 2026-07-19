use super::super::{GeneratedAuthorityShardParseRequest, parse_shard};
use super::shard_samples::SourceDefinitionSample;

fn rejection_code(bytes: &[u8]) -> &'static str {
    parse_shard(GeneratedAuthorityShardParseRequest { bytes })
        .expect_err("shard must be rejected")
        .stable_text()
}

#[test]
fn rejects_unknown_fields_trailing_data_and_old_schema() {
    let unknown = SourceDefinitionSample {
        extra_field: ",\"unknown\":true",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&unknown.bytes()),
        "generated_authority_shard_json_invalid"
    );

    let trailing = SourceDefinitionSample {
        trailing: "{}",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&trailing.bytes()),
        "generated_authority_shard_trailing_data"
    );

    let old = SourceDefinitionSample {
        schema: "GeneratedSurfaceAuthorityShard-v0",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&old.bytes()),
        "generated_authority_shard_schema_unsupported"
    );
}

#[test]
fn rejects_cycles_unsorted_sources_and_command_substitution() {
    let cycle = SourceDefinitionSample {
        output: "source/input.json",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&cycle.bytes()),
        "generated_authority_shard_source_projection_cycle"
    );

    let unsorted = SourceDefinitionSample {
        sources: "\"z/input.json\",\"a/input.json\"",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&unsorted.bytes()),
        "generated_authority_paths_noncanonical"
    );

    let command = SourceDefinitionSample {
        command: "different-tool write",
        ..Default::default()
    };
    assert_eq!(
        rejection_code(&command.bytes()),
        "generated_authority_command_invalid"
    );
}

#[test]
fn rejects_noncanonical_retained_replacement_targets() {
    for target in ["HCT-", "PS-", "HCT-lower", "PS--X"] {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schema_version": "GeneratedSurfaceAuthorityShard-v1",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [{
                "disposition": "retained_context",
                "output": "generated/context.json",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "reason": "context",
                "replacement_targets": [target],
                "preserve": true,
                "physical_deletion_authorized": false
            }]
        }))
        .unwrap();
        assert_eq!(
            rejection_code(&bytes),
            "generated_authority_shard_retained_context_invalid",
            "target {target} must fail closed"
        );
    }
}

#[test]
fn rejects_adopted_contract_with_nonsemantic_amendment() {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema_version": "GeneratedSurfaceAuthorityShard-v1",
        "contract_id": "harness-ultragoal-successor-contract-v2",
        "surfaces": [{
            "disposition": "adopted_schema_contract",
            "output": "examples/generated/product.json",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "schema": "schemas/product.json",
            "schema_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_contract": "GOAL_CONTRACT.md",
            "source_contract_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "amendment_log": "AMENDMENTS.jsonl",
            "amendment_id": "amendment-three",
            "amendment_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "claim_ceiling": "contract_authority_only"
        }]
    }))
    .unwrap();
    assert_eq!(
        rejection_code(&bytes),
        "generated_authority_adopted_schema_contract_invalid"
    );
}
