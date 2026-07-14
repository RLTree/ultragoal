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
