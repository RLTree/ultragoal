// This is the direct source closure for API and command activation. It binds
// declaration, registry construction, exact-row gating, status finalization,
// public serialization, and the production summary export. Generic context and
// confined-I/O primitives remain covered by the accepted context dependency.
const WITNESS_SOURCES: &[WitnessSource] = &[
    source!(
        "validator/src/api_witness.rs",
        include_bytes!("../../../api_witness.rs"),
        "compiled API declaration"
    ),
    source!(
        "validator/src/command_witness.rs",
        include_bytes!("../../../command_witness.rs"),
        "compiled command row digest"
    ),
    source!(
        "validator/src/cli/successor/catalog/mod.rs",
        include_bytes!("../../../cli/successor/catalog/mod.rs"),
        "command catalog composition"
    ),
    source!(
        "validator/src/cli/successor/catalog/evaluation_and_migration.rs",
        include_bytes!("../../../cli/successor/catalog/evaluation_and_migration.rs"),
        "evaluation and migration command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/repository_fit_and_checks.rs",
        include_bytes!("../../../cli/successor/catalog/repository_fit_and_checks.rs"),
        "repository fit and validation command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/inspection.rs",
        include_bytes!("../../../cli/successor/catalog/inspection.rs"),
        "inspection command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/observability_and_package.rs",
        include_bytes!("../../../cli/successor/catalog/observability_and_package.rs"),
        "observability and package command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/options.rs",
        include_bytes!("../../../cli/successor/catalog/options.rs"),
        "command option contracts"
    ),
    source!(
        "validator/src/cli/successor/command_contract/mod.rs",
        include_bytes!("../../../cli/successor/command_contract/mod.rs"),
        "command model composition"
    ),
    source!(
        "validator/src/cli/successor/command_contract/arguments.rs",
        include_bytes!("../../../cli/successor/command_contract/arguments.rs"),
        "typed command arguments"
    ),
    source!(
        "validator/src/cli/successor/command_contract/commands.rs",
        include_bytes!("../../../cli/successor/command_contract/commands.rs"),
        "command group and action model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/descriptor.rs",
        include_bytes!("../../../cli/successor/command_contract/descriptor.rs"),
        "command descriptor model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/exit.rs",
        include_bytes!("../../../cli/successor/command_contract/exit.rs"),
        "command effect and exit model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/invocation.rs",
        include_bytes!("../../../cli/successor/command_contract/invocation.rs"),
        "parsed invocation model"
    ),
    source!(
        "validator/src/cli/successor_public/operation_binding/mod.rs",
        include_bytes!("../../../cli/successor_public/operation_binding/mod.rs"),
        "supported operation activation authority"
    ),
    source!(
        "validator/src/cli/successor_public/operation_binding/groups.rs",
        include_bytes!("../../../cli/successor_public/operation_binding/groups.rs"),
        "complete command-group activation authority"
    ),
    source!(
        "validator/src/cli/successor_public/operation_binding/package_inventory.rs",
        include_bytes!("../../../cli/successor_public/operation_binding/package_inventory.rs"),
        "package inventory operation API authority"
    ),
    source!(
        "validator/src/cli/successor_public/operation_binding/package_install_test.rs",
        include_bytes!("../../../cli/successor_public/operation_binding/package_install_test.rs"),
        "isolated package installation operation API authority"
    ),
    source!(
        "validator/src/cli/successor_public/output_limit.rs",
        include_bytes!("../../../cli/successor_public/output_limit.rs"),
        "public dispatcher consumption of operation authority"
    ),
    source!(
        "validator/src/cli/successor_public/package_inventory/mod.rs",
        include_bytes!("../../../cli/successor_public/package_inventory/mod.rs"),
        "package inventory production dispatch"
    ),
    source!(
        "validator/src/cli/successor_public/package_install_test/mod.rs",
        include_bytes!("../../../cli/successor_public/package_install_test/mod.rs"),
        "isolated package installation production dispatch"
    ),
    source!(
        "validator/src/distribution/filesystem/root/workspace_context.rs",
        include_bytes!("../../../distribution/filesystem/root/workspace_context.rs"),
        "workspace-bound package output authority"
    ),
    source!(
        "validator/src/distribution/package/product/inventory_publication.rs",
        include_bytes!("../../../distribution/package/product/inventory_publication.rs"),
        "atomic package inventory publication"
    ),
    source!(
        "validator/src/cli/successor_public/evaluation/run.rs",
        include_bytes!("../../../cli/successor_public/evaluation/run.rs"),
        "evaluation run unsupported-capability route"
    ),
    source!(
        "validator/src/cli/successor_public/orchestration.rs",
        include_bytes!("../../../cli/successor_public/orchestration.rs"),
        "bounded public orchestration projection"
    ),
    source!(
        "validator/src/inventory/mod.rs",
        include_bytes!("../../mod.rs"),
        "public inventory export"
    ),
    source!(
        "validator/src/inventory/builder.rs",
        include_bytes!("../../builder.rs"),
        "guard ordering and catalog finalization"
    ),
    source!(
        "validator/src/inventory/digest.rs",
        include_bytes!("../../digest.rs"),
        "activation row digest"
    ),
    source!(
        "validator/src/inventory/fs.rs",
        include_bytes!("../../fs.rs"),
        "registry source read gate"
    ),
    source!(
        "validator/src/inventory/projection.rs",
        include_bytes!("../../projection.rs"),
        "catalog projection export"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/mod.rs",
        include_bytes!("mod.rs"),
        "exact activation guard"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/guard.rs",
        include_bytes!("guard.rs"),
        "frontier-aware activation decision"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/exact_source_manifest.rs",
        include_bytes!("exact_source_manifest.rs"),
        "activation source and row verifier"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/activation_rows.rs",
        include_bytes!("activation_rows.rs"),
        "exact activation row reconciliation"
    ),
    source!(
        "validator/src/inventory/registry/data.rs",
        include_bytes!("../data.rs"),
        "registry identifier and row storage"
    ),
    source!(
        "validator/src/inventory/registry/integrity.rs",
        include_bytes!("../integrity.rs"),
        "adopted registry integrity gate"
    ),
    registry_source!("frontier/mod.rs", "frontier authority"),
    registry_source!("frontier/inspection.rs", "bounded frontier projection"),
    registry_source!(
        "frontier/validated_snapshot.rs",
        "shared validated frontier snapshot"
    ),
    registry_source!("frontier/change_impact.rs", "change authority"),
    registry_source!("frontier/envelope_codec.rs", "envelope integrity"),
    registry_source!("frontier/handoff_adjacency.rs", "handoff authority"),
    registry_source!("frontier/lifecycle.rs", "lane lifecycle authority"),
    registry_source!("frontier/lease_issuance/mod.rs", "lease authority"),
    registry_source!(
        "frontier/lease_issuance/worktree_identity.rs",
        "active worktree identity authority"
    ),
    registry_source!(
        "frontier/lease_issuance/debt_worktree/mod.rs",
        "P0 worktree lease authority"
    ),
    registry_source!(
        "frontier/lease_issuance/debt_worktree/diagnostic_source.rs",
        "P0 diagnostic source authority"
    ),
    registry_source!("frontier/scope_ownership.rs", "scope authority"),
    registry_source!("frontier/scope_consumption.rs", "consumption authority"),
    registry_source!("load.rs", "frontier-bound registry construction"),
    registry_source!("mod.rs", "registry load and guard export"),
    registry_source!("semantic.rs", "active API row construction"),
    source!(
        "validator/src/inventory/registry/sources.rs",
        include_bytes!("../sources.rs"),
        "registry source coverage gate"
    ),
    source!(
        "validator/src/inventory/registry/topology.rs",
        include_bytes!("../topology.rs"),
        "candidate command row construction"
    ),
    source!(
        "validator/src/inventory/types/mod.rs",
        include_bytes!("../../types/mod.rs"),
        "active status and catalog serialization"
    ),
    source!(
        "validator/src/inventory/validate/duplicates.rs",
        include_bytes!("../../validate/duplicates.rs"),
        "duplicate activation classification"
    ),
    source!(
        "validator/src/inventory/validate/mod.rs",
        include_bytes!("../../validate/mod.rs"),
        "catalog status reconciliation"
    ),
    source!(
        "validator/examples/hct_inventory.rs",
        include_bytes!("../../../../examples/hct_inventory.rs"),
        "production catalog and closure export"
    ),
];
