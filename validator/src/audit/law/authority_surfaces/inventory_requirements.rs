use std::collections::BTreeSet;
use std::path::Path;

const REQUIRED_SURFACES: &[RequiredSurface] = &[
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/mod.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/registry.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/source/mod.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/source/output.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/source/raw.rs",
    ),
    surface(
        "source",
        "validator/src/audit/law/authority_surfaces/inventory_requirements.rs",
    ),
    surface(
        "source",
        "validator/src/audit/namespace/source/identifiers.rs",
    ),
    surface(
        "source",
        "validator/src/audit/namespace/source/path_labels.rs",
    ),
    surface("schema", "schemas/mandatory-law-surfaces.schema.json"),
    surface("schema", "schemas/red-packet.schema.json"),
    surface("schema", "schemas/final-packet-proof.schema.json"),
    surface("schema", "schemas/cli-control-plane-receipt.schema.json"),
    surface(
        "schema",
        "schemas/research-article-to-law-trace.schema.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/authority-source-binding.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/generated-proof-artifact-provenance-anti-fabrication.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/distinct-proof-surfaces-claim-ceilings.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/total-authority-types-impossible-state-elimination.json",
    ),
    surface(
        "valid_fixture",
        "fixtures/mandatory-law-surfaces/valid/validator-theater-miswire-resistance.json",
    ),
    surface("red_fixture_catalog", "templates/RED_FIXTURES.json"),
    surface("package_inventory", "plugin-manifest-draft.json"),
    surface("package_inventory", "docs/plugin-cohesion-manifest.json"),
    surface(
        "generated_artifact",
        "docs/generated/observability/command-inventory.json",
    ),
    surface(
        "receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
    ),
    surface(
        "receipt",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
    runtime_surface(
        "receipt",
        "validation_artifacts/coverage/coverage-receipt.json",
    ),
    surface("standards", "templates/agent-standards/enforcement.json"),
    surface(
        "standards",
        "templates/agent-standards/enforcement-audit.tsv",
    ),
    surface("source_obligation", "docs/source-obligation-matrix.json"),
    surface("source_obligation", "docs/source-obligation-matrix.md"),
    surface(
        "foundational_trace",
        "docs/foundational-law-traceability.json",
    ),
    surface(
        "claim_guard",
        "validator/src/audit/final_packet/observability/mod.rs",
    ),
    surface(
        "claim_guard",
        "validator/src/cli/control/plane/proof/transaction.rs",
    ),
    surface(
        "claim_guard",
        "validator/src/cli/control/plane/proof/diagnostic.rs",
    ),
    surface(
        "final_packet_blocker",
        "validation_artifacts/review/final-packet-proof.json",
    ),
    surface(
        "update_goal_blocker",
        "validation_artifacts/cli/update-goal-eligibility.json",
    ),
];

#[derive(Clone, Copy)]
struct RequiredSurface {
    role: &'static str,
    rel: &'static str,
    package_inventory_required: bool,
}

const fn surface(role: &'static str, rel: &'static str) -> RequiredSurface {
    RequiredSurface {
        role,
        rel,
        package_inventory_required: true,
    }
}

const fn runtime_surface(role: &'static str, rel: &'static str) -> RequiredSurface {
    RequiredSurface {
        role,
        rel,
        package_inventory_required: false,
    }
}

pub(super) fn failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for surface in REQUIRED_SURFACES {
        if !root.join(surface.rel).is_file() {
            push(
                &mut out,
                "authority-source-binding",
                format!(
                    "authority_surface_missing:role={}:path={}",
                    surface.role, surface.rel
                ),
            );
        }
        if surface.package_inventory_required && !inventory.contains(surface.rel) {
            push(
                &mut out,
                "authority-source-binding",
                format!(
                    "authority_surface_not_in_package_inventory:role={}:path={}",
                    surface.role, surface.rel
                ),
            );
        }
    }
    out
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
