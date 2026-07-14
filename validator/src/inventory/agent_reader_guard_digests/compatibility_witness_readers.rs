pub(crate) const COMPATIBILITY_WITNESS_READERS: &[ReaderSpec] = &[
    reader!(
        "validator/src/inventory/compatibility/agent/specs.rs",
        "compatibility/agent_specs.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/agent/witness.rs",
        "compatibility/agent_witness.rs",
        false
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness/mod.rs",
        "compatibility/reader_witness/mod.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness/scan/mod.rs",
        "compatibility/reader_witness/scan/mod.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness/scan/literals.rs",
        "compatibility/reader_witness/scan/literals.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness/scan/unicode.rs",
        "compatibility/reader_witness/scan/unicode.rs",
        true
    ),
    reader!("validator/src/inventory/walk/mod.rs", "walk/mod.rs", false),
    // These orchestration guards mention the canonical host-owned roots only
    // to reject worker authority. They do not read agent descriptors. Exact
    // compile-time bytes keep that negative surface candidate-bound.
    reader!(
        "validator/src/orchestration/artifact/mod.rs",
        "../orchestration/artifact/mod.rs",
        false
    ),
    reader!(
        "validator/src/orchestration/scope_policy.rs",
        "../orchestration/scope_policy.rs",
        false
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness_specs.rs",
        "compatibility/reader_witness_specs.rs",
        false
    ),
];
