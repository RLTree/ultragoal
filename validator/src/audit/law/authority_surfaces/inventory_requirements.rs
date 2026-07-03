use std::collections::BTreeSet;
use std::path::Path;

macro_rules! package_authority_surface {
    ($role:literal, $rel:literal $(,)?) => {
        RequiredSurface {
            role: $role,
            rel: $rel,
            package_inventory_required: true,
        }
    };
}

macro_rules! runtime_authority_surface {
    ($role:literal, $rel:literal $(,)?) => {
        RequiredSurface {
            role: $role,
            rel: $rel,
            package_inventory_required: false,
        }
    };
}

const REQUIRED_SURFACES: &[RequiredSurface] = &[
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/mod.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/registry.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/source/mod.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/source/output.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/source/raw/mod.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/authority_labels.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory_requirements.rs",
    ),
    package_authority_surface!("source", "validator/src/audit/namespace/classes.rs"),
    package_authority_surface!("source", "validator/src/audit/namespace/law/mod.rs"),
    package_authority_surface!("source", "validator/src/audit/namespace/law/path_rules.rs"),
    package_authority_surface!("source", "validator/src/audit/namespace/mod.rs"),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/failure_text.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/identifiers.rs",
    ),
    package_authority_surface!("source", "validator/src/audit/namespace/source/mod.rs"),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/path_labels.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/label_patterns.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/semantic_tokens.rs",
    ),
    package_authority_surface!(
        "source",
        "validator/src/audit/namespace/source/string_labels.rs",
    ),
    package_authority_surface!("source", "validator/src/audit/namespace/source/topology.rs",),
    package_authority_surface!("schema", "schemas/mandatory-law-surfaces.schema.json"),
    package_authority_surface!("schema", "schemas/red-packet.schema.json"),
    package_authority_surface!("schema", "schemas/final-packet-proof.schema.json"),
    package_authority_surface!("schema", "schemas/cli-control-plane-receipt.schema.json"),
    package_authority_surface!(
        "schema",
        "schemas/research-article-to-law-trace.schema.json",
    ),
    package_authority_surface!("law_registry", "docs/mandatory-law-surfaces.json"),
    package_authority_surface!("research_registry", "docs/research-source-registry.json"),
    package_authority_surface!("research_trace", "docs/research-article-to-law-trace.json"),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/authority-source-binding.json",
    ),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
    ),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/generated-proof-artifact-provenance-anti-fabrication.json",
    ),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/distinct-proof-surfaces-claim-ceilings.json",
    ),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/total-authority-types-impossible-state-elimination.json",
    ),
    package_authority_surface!(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/validator-theater-miswire-resistance.json",
    ),
    package_authority_surface!("red_fixture_catalog", "templates/RED_FIXTURES.json"),
    package_authority_surface!("package_manifest", ".codex-plugin/plugin.json"),
    package_authority_surface!("package_inventory", "plugin-manifest-draft.json"),
    package_authority_surface!("package_inventory", "docs/plugin-cohesion-manifest.json"),
    package_authority_surface!("setup_retrofit_output", ".codex/setup-worktree-env.sh"),
    package_authority_surface!(
        "generated_artifact",
        "docs/generated/observability/command-inventory.json",
    ),
    package_authority_surface!(
        "receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
    ),
    package_authority_surface!(
        "receipt",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
    runtime_authority_surface!(
        "receipt",
        "validation_artifacts/coverage/coverage-receipt.json",
    ),
    package_authority_surface!("standards", "templates/agent-standards/enforcement.json"),
    package_authority_surface!("standards", "templates/agent-standards/enforcement.tsv"),
    package_authority_surface!(
        "standards",
        "templates/agent-standards/enforcement-audit.tsv",
    ),
    package_authority_surface!("source_obligation", "docs/source-obligation-matrix.json"),
    package_authority_surface!("source_obligation", "docs/source-obligation-matrix.md"),
    package_authority_surface!(
        "foundational_trace",
        "docs/foundational-law-traceability.json",
    ),
    package_authority_surface!(
        "claim_guard",
        "validator/src/audit/final_packet/observability/mod.rs",
    ),
    package_authority_surface!(
        "claim_guard",
        "validator/src/cli/control/plane/proof/transaction.rs",
    ),
    package_authority_surface!(
        "claim_guard",
        "validator/src/cli/control/plane/proof/diagnostic.rs",
    ),
    package_authority_surface!(
        "final_packet_blocker",
        "validation_artifacts/review/final-packet-proof.json",
    ),
    package_authority_surface!(
        "update_goal_blocker",
        "validation_artifacts/cli/update-goal-eligibility.json",
    ),
];

#[derive(Clone, Copy)]
pub(super) struct RequiredSurface {
    pub(super) role: &'static str,
    pub(super) rel: &'static str,
    pub(super) package_inventory_required: bool,
}

pub(super) fn failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for surface in super::surface_inventory::rows(root, inventory) {
        if !surface.exists_on_disk {
            push(
                &mut out,
                "authority-source-binding",
                format!(
                    "authority_surface_missing:role={}:path={}",
                    surface.role, surface.path
                ),
            );
        }
        if surface.package_inventory_required && !surface.listed_in_package_inventory {
            push(
                &mut out,
                "authority-source-binding",
                format!(
                    "authority_surface_not_in_package_inventory:role={}:path={}",
                    surface.role, surface.path
                ),
            );
        }
    }
    out
}

pub(super) fn required_surfaces() -> &'static [RequiredSurface] {
    REQUIRED_SURFACES
}

#[cfg(test)]
pub(crate) fn required_surfaces_for_test() -> Vec<(&'static str, &'static str, bool)> {
    REQUIRED_SURFACES
        .iter()
        .map(|surface| {
            (
                surface.role,
                surface.rel,
                surface.package_inventory_required,
            )
        })
        .collect()
}

fn push(out: &mut Vec<(String, String)>, check: &str, detail: String) {
    out.push((check.to_string(), detail));
}
