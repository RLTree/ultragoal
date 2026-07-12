#[derive(Clone, Copy)]
pub(crate) struct ReaderSpec {
    pub(crate) path: &'static str,
    pub(crate) bytes: &'static [u8],
    pub(crate) legacy_tokens_are_negative_only: bool,
}

macro_rules! reader {
    ($path:literal, $include:literal, $negative:literal) => {
        ReaderSpec {
            path: $path,
            bytes: include_bytes!($include),
            legacy_tokens_are_negative_only: $negative,
        }
    };
}

// This compiled allowlist is deliberately outside inventory/compatibility. The
// witness scans that subtree too, and compares this source file with its own
// compile-time bytes before trusting any row below.
pub(crate) const READERS: &[ReaderSpec] = &[
    reader!(
        "validator/src/agent_manifest.rs",
        "../agent_manifest.rs",
        false
    ),
    reader!("validator/src/agent_roles.rs", "../agent_roles.rs", false),
    reader!(
        "validator/src/audit/contract.rs",
        "../audit/contract.rs",
        false
    ),
    reader!(
        "validator/src/audit/coverage/scope/roots.rs",
        "../audit/coverage/scope/roots.rs",
        true
    ),
    reader!(
        "validator/src/audit/plugin/product/cohesion/manifest.rs",
        "../audit/plugin/product/cohesion/manifest.rs",
        false
    ),
    reader!(
        "validator/src/audit/plugin/product/cohesion/mod.rs",
        "../audit/plugin/product/cohesion/mod.rs",
        false
    ),
    reader!(
        "validator/src/inventory/components.rs",
        "components.rs",
        false
    ),
    reader!(
        "validator/src/inventory/registry/topology.rs",
        "registry/topology.rs",
        false
    ),
    reader!(
        "validator/src/package/inventory/mod.rs",
        "../package/inventory/mod.rs",
        false
    ),
    reader!(
        "validator/src/claim_semantics/plugin_policy/mod.rs",
        "../claim_semantics/plugin_policy/mod.rs",
        true
    ),
    reader!(
        "validator/src/claim_semantics/coverage/receipt/exclusions.rs",
        "../claim_semantics/coverage/receipt/exclusions.rs",
        true
    ),
    reader!(
        "validator/src/claim_semantics/retired_reviewer_policy.rs",
        "../claim_semantics/retired_reviewer_policy.rs",
        true
    ),
    reader!(
        "validator/src/cli/control/plane/registry/mod.rs",
        "../cli/control/plane/registry/mod.rs",
        false
    ),
    reader!(
        "validator/src/cli/control/plane/registry/agent_rows.rs",
        "../cli/control/plane/registry/agent_rows.rs",
        true
    ),
    reader!(
        "validator/src/cli/live_loop/surfaces/input_spec/path_rules.rs",
        "../cli/live_loop/surfaces/input_spec/path_rules.rs",
        true
    ),
    reader!(
        "validator/src/audit/plugin/registry/mod.rs",
        "../audit/plugin/registry/mod.rs",
        false
    ),
    reader!(
        "validator/src/audit/plugin/registry/live/mod.rs",
        "../audit/plugin/registry/live/mod.rs",
        true
    ),
    reader!(
        "validator/src/audit/plugin/registry/live/raw.rs",
        "../audit/plugin/registry/live/raw.rs",
        true
    ),
    reader!(
        "validator/src/audit/plugin/registry/live/reviewers.rs",
        "../audit/plugin/registry/live/reviewers.rs",
        false
    ),
    // These three production modules jointly read, decode, and validate the
    // current source and host agent-authority catalogs. They are positive
    // readers, so their exact bytes are bound without a negative-only escape.
    reader!(
        "validator/src/plugin_product/agent_discovery/host.rs",
        "../plugin_product/agent_discovery/host.rs",
        false
    ),
    reader!(
        "validator/src/plugin_product/agent_discovery/model.rs",
        "../plugin_product/agent_discovery/model.rs",
        false
    ),
    reader!(
        "validator/src/plugin_product/agent_discovery/source.rs",
        "../plugin_product/agent_discovery/source.rs",
        false
    ),
    reader!(
        "validator/src/review/round/config.rs",
        "../review/round/config.rs",
        false
    ),
    reader!(
        "validator/src/review/round/personas.rs",
        "../review/round/personas.rs",
        false
    ),
    reader!(
        "validator/src/review/round/registry.rs",
        "../review/round/registry.rs",
        true
    ),
    reader!(
        "validator/src/review/round/registry/reader.rs",
        "../review/round/registry/reader.rs",
        false
    ),
    reader!(
        "validator/src/review/round/registry/reader/json.rs",
        "../review/round/registry/reader/json.rs",
        false
    ),
    reader!(
        "validator/src/review/round/registry/reader/path.rs",
        "../review/round/registry/reader/path.rs",
        false
    ),
    reader!(
        "validator/src/review/round/registry/semantics.rs",
        "../review/round/registry/semantics.rs",
        false
    ),
    reader!(
        "validator/src/review/round/registry/validation.rs",
        "../review/round/registry/validation.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/agent_specs.rs",
        "compatibility/agent_specs.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/agent_witness.rs",
        "compatibility/agent_witness.rs",
        false
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness.rs",
        "compatibility/reader_witness.rs",
        true
    ),
    reader!(
        "validator/src/inventory/compatibility/reader_witness/scan.rs",
        "compatibility/reader_witness/scan.rs",
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
    reader!("validator/src/inventory/walk.rs", "walk.rs", false),
    // These orchestration guards mention the canonical host-owned roots only
    // to reject worker authority. They do not read agent descriptors. Exact
    // compile-time bytes keep that negative surface candidate-bound.
    reader!(
        "validator/src/orchestration/artifact.rs",
        "../orchestration/artifact.rs",
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
    reader!(
        "validator/src/inventory/discovery.rs",
        "discovery.rs",
        false
    ),
    reader!(
        "validator/src/inventory/legacy/matchers.rs",
        "legacy/matchers.rs",
        true
    ),
    reader!(
        "validator/src/inventory/legacy/scope.rs",
        "legacy/scope.rs",
        false
    ),
    reader!(
        "validator/src/inventory/plugin_manifest_hooks.rs",
        "plugin_manifest_hooks.rs",
        false
    ),
    reader!(
        "validator/src/inventory/agent_reader_guard_manifest_digests.rs",
        "agent_reader_guard_manifest_digests.rs",
        false
    ),
    reader!("validator/src/inventory/validate.rs", "validate.rs", false),
    reader!("validator/src/lib.rs", "../lib.rs", false),
    reader!("validator/src/package/mod.rs", "../package/mod.rs", false),
    reader!(
        "validator/src/red/fixture/package.rs",
        "../red/fixture/package.rs",
        false
    ),
    reader!(
        "validator/src/review/materiality.rs",
        "../review/materiality.rs",
        false
    ),
    reader!(
        "validator/src/review/materiality/registry.rs",
        "../review/materiality/registry.rs",
        false
    ),
    reader!(
        "validator/src/review/round/report.rs",
        "../review/round/report.rs",
        false
    ),
    reader!(
        "validator/src/schema_catalog/fixture_schema_rules.rs",
        "../schema_catalog/fixture_schema_rules.rs",
        true
    ),
    reader!(
        "validator/src/schema_catalog/mod.rs",
        "../schema_catalog/mod.rs",
        false
    ),
    reader!(
        "validator/src/schema_catalog/schema/patterns.rs",
        "../schema_catalog/schema/patterns.rs",
        true
    ),
    reader!(
        "validator/src/target_repo/baseline.rs",
        "../target_repo/baseline.rs",
        false
    ),
    reader!(
        "validator/src/target_repo/baseline_mode.rs",
        "../target_repo/baseline_mode.rs",
        false
    ),
];

#[path = "agent_reader_guard_manifest_digests.rs"]
mod manifests;
pub(crate) use manifests::MANIFESTS;
#[path = "agent_reader_guard_package_digests.rs"]
mod package;
pub(crate) use package::PACKAGE_READERS;

pub(crate) fn all_readers() -> impl Iterator<Item = &'static ReaderSpec> {
    READERS.iter().chain(PACKAGE_READERS)
}
