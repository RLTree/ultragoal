use super::inventory_requirements::RequiredSurface;

pub(super) const REQUIRED_SURFACES: &[RequiredSurface] = &[
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory/mod.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/mod.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/registry.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/source/mod.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/source/output.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/source/raw/mod.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/authority_labels.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory/requirements.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory/required_surfaces.rs",
    ),
    package(
        "source",
        "validator/src/audit/law/authority_surfaces/observability_package_surfaces.rs",
    ),
    package("output_authority", "validator/src/output_path.rs"),
    package("source", "validator/src/audit/namespace/classes.rs"),
    package("source", "validator/src/audit/namespace/law/mod.rs"),
    package("source", "validator/src/audit/namespace/law/path_rules.rs"),
    package("source", "validator/src/audit/namespace/mod.rs"),
    package(
        "source",
        "validator/src/audit/namespace/source/failure_text.rs",
    ),
    package(
        "source",
        "validator/src/audit/namespace/source/identifiers.rs",
    ),
    package("source", "validator/src/audit/namespace/source/mod.rs"),
    package(
        "source",
        "validator/src/audit/namespace/source/path_labels.rs",
    ),
    package(
        "source",
        "validator/src/audit/namespace/source/label_patterns.rs",
    ),
    package(
        "source",
        "validator/src/audit/namespace/source/semantic_tokens.rs",
    ),
    package(
        "source",
        "validator/src/audit/namespace/source/string_labels.rs",
    ),
    package(
        "source",
        "validator/src/audit/namespace/source/topology/mod.rs",
    ),
    package("schema", "schemas/mandatory-law-surfaces.schema.json"),
    package("schema", "schemas/red-packet.schema.json"),
    package("schema", "schemas/final-packet-proof.schema.json"),
    package("schema", "schemas/cli-control-plane-receipt.schema.json"),
    package(
        "schema",
        "schemas/research-article-to-law-trace.schema.json",
    ),
    package("law_registry", "docs/mandatory-law-surfaces.json"),
    package(
        "namespace_class_registry",
        "docs/namespace-class-registry.json",
    ),
    package("research_registry", "docs/research-source-registry.json"),
    package("research_trace", "docs/research-article-to-law-trace.json"),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/authority-source-binding.json",
    ),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
    ),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/generated-proof-artifact-provenance-anti-fabrication.json",
    ),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/distinct-proof-surfaces-claim-ceilings.json",
    ),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/total-authority-types-impossible-state-elimination.json",
    ),
    package(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/validator-theater-miswire-resistance.json",
    ),
    package("red_fixture_catalog", "templates/RED_FIXTURES.json"),
    package("package_manifest", ".codex-plugin/plugin.json"),
    package("package_inventory", "plugin-manifest-draft.json"),
    package("package_inventory", "docs/plugin-cohesion-manifest.json"),
    package(
        "orchestration_authority",
        ".codex/automations/ultragoal-orchestrator/automation.toml",
    ),
    package(
        "orchestration_authority",
        ".codex/automations/ultragoal-orchestrator/transition-receipt.json",
    ),
    package("setup_retrofit_output", ".codex/setup-worktree-env.sh"),
    runtime(
        "receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
    ),
    runtime(
        "receipt",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
    runtime(
        "receipt",
        "validation_artifacts/coverage/coverage-receipt.json",
    ),
    runtime(
        "runtime_receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
    ),
    runtime(
        "runtime_receipt",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
    runtime(
        "runtime_receipt",
        "validation_artifacts/coverage/coverage-receipt.json",
    ),
    package("standards", "templates/agent-standards/enforcement.json"),
    package("standards", "templates/agent-standards/enforcement.tsv"),
    package(
        "standards",
        "templates/agent-standards/enforcement-audit.tsv",
    ),
    package("source_obligation", "docs/source-obligation-matrix.json"),
    package("source_obligation", "docs/source-obligation-matrix.md"),
    package(
        "foundational_trace",
        "docs/foundational-law-traceability.json",
    ),
    package(
        "claim_guard",
        "validator/src/audit/final_packet/observability/mod.rs",
    ),
    package(
        "claim_guard",
        "validator/src/audit/cli/control_plane/authority/receipt/mod.rs",
    ),
    package(
        "claim_guard",
        "validator/src/audit/cli/control_plane/authority/mod.rs",
    ),
    runtime(
        "final_packet_blocker",
        "validation_artifacts/review/final-packet-proof.json",
    ),
    runtime(
        "update_goal_blocker",
        "validation_artifacts/cli/update-goal-eligibility.json",
    ),
];

const fn package(role: &'static str, rel: &'static str) -> RequiredSurface {
    RequiredSurface {
        role,
        rel,
        package_inventory_required: true,
        existence_required: true,
    }
}

const fn runtime(role: &'static str, rel: &'static str) -> RequiredSurface {
    RequiredSurface {
        role,
        rel,
        package_inventory_required: false,
        existence_required: false,
    }
}
