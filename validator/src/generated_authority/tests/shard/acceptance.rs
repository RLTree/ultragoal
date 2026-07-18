use super::super::{GeneratedAuthorityShardParseRequest, parse_shard};
use std::collections::BTreeSet;

const SHARDS: &[&[u8]] = &[
    include_bytes!(
        "../../../../../migration/generated-surface-authority/agent-standards-documents.json"
    ),
    include_bytes!(
        "../../../../../migration/generated-surface-authority/agent-standards-indexes.json"
    ),
    include_bytes!("../../../../../migration/generated-surface-authority/dependency-locks.json"),
    include_bytes!(
        "../../../../../migration/generated-surface-authority/retained-predecessor-context.json"
    ),
];

#[test]
fn current_shards_are_canonical_and_have_unique_outputs() {
    let mut outputs = BTreeSet::new();
    for bytes in SHARDS {
        let response = parse_shard(GeneratedAuthorityShardParseRequest { bytes })
            .expect("current shard parses");
        assert_eq!(&response.canonical_bytes, bytes);
        for output in response.definitions.keys() {
            assert!(
                outputs.insert(output.clone()),
                "duplicate output across shards"
            );
        }
    }
}
