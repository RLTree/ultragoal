use crate::schema_catalog::SchemaStore;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn run(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    source_and_standards(root, store, failures);
    provenance(root, store, failures);
    control_and_observability(root, failures);
    for failure in crate::audit::text_guards::private_home_path_failures(root) {
        push(failures, "plugin-inventory-closure", failure);
    }
    mandatory_surfaces(root, check_ids, failures);
}

fn source_and_standards(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    for failure in crate::audit::text_guards::source_card_freshness_failures(root) {
        push(failures, "source-card-freshness", failure);
    }
    for failure in crate::audit::source_obligations::failures(root) {
        push(failures, "source-obligation-coverage", failure);
    }
    for failure in crate::audit::law::surface::receipts::package_failures(root) {
        push(failures, "source-obligation-coverage", failure);
    }
    for (check, failure) in crate::audit::law::authority_surfaces::package_failures(root) {
        push(failures, &check, failure);
    }
    for failure in crate::audit::standards_gardening::failures(root, store) {
        push(failures, "standards-gardener-promotion", failure);
    }
    for failure in crate::audit::agent::standards::enforcement::failures(root) {
        push(failures, "agent-standards-enforcement", failure);
    }
    for failure in crate::audit::template_integrity::package_failures(root) {
        push(failures, "agent-standards-enforcement", failure);
    }
    for failure in crate::audit::coverage::scope::package_failures(root) {
        push(failures, "agent-standards-enforcement", failure);
    }
    for failure in crate::audit::plugin::product::cohesion::package_failures(root) {
        push(failures, "agent-standards-enforcement", failure);
    }
    for failure in crate::audit::product::fitness::package_failures(root) {
        push(failures, "product-fitness-proof", failure);
    }
}

fn provenance(root: &Path, store: &SchemaStore, failures: &mut BTreeMap<String, Vec<String>>) {
    for failure in crate::audit::text_guards::moving_value_drift_failures(root) {
        push(failures, "validator-execution-provenance", failure);
    }
    for failure in crate::audit::text_guards::stale_review_law_failures(root) {
        push(failures, "validator-execution-provenance", failure);
    }
    for failure in crate::audit::session_log_hardening::package_failures(root, store) {
        push(failures, "validator-execution-provenance", failure);
    }
    for failure in crate::audit::final_packet::claim_guard_failures(root, store) {
        push(failures, "validator-execution-provenance", failure);
    }
    for failure in crate::audit::plugin::laws::package_failures(root, store) {
        push(failures, "validator-execution-provenance", failure);
    }
}

fn control_and_observability(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    for failure in crate::audit::cli::control_plane::authority::package_failures(root) {
        push(failures, control_plane_check(&failure), failure);
    }
    for failure in crate::audit::cli::performance::package_failures(root) {
        push(
            failures,
            "cli-performance-latency-speed-iteration-fitness",
            failure,
        );
    }
    for (check, failure) in crate::audit::rust::developer::package_failures(root) {
        push(failures, &check, failure);
    }
    for failure in crate::audit::observability::package_failures(root) {
        push(
            failures,
            "full-local-observability-stack-integration-non-opaque-failure",
            failure,
        );
    }
    for failure in crate::audit::openai::package_failures(root) {
        push(
            failures,
            "openai-api-key-model-cost-external-ai-boundary",
            failure,
        );
    }
    for failure in crate::audit::promptfoo::package_failures(root) {
        push(
            failures,
            "promptfoo-eval-red-team-provider-separation",
            failure,
        );
    }
    for failure in crate::audit::halo::package_failures(root) {
        push(failures, "halo-ranked-harness-change-optimization", failure);
    }
}

fn mandatory_surfaces(
    root: &Path,
    check_ids: &[String],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let current_failures = failures.clone();
    for failure in crate::audit::mandatory::law::surfaces::package_failures_with_current(
        root,
        &current_failures,
    ) {
        if let Some(check_id) = mandatory_law_check_id(&failure, check_ids) {
            push(failures, &check_id, failure.clone());
        }
        push(failures, "source-obligation-coverage", failure);
    }
}

fn control_plane_check(failure: &str) -> &'static str {
    if failure.contains("self-law")
        || failure.contains("self_law")
        || failure.contains("cli-self-law-compliance")
    {
        "cli-self-law-compliance"
    } else {
        "cli-control-plane-authority"
    }
}

fn mandatory_law_check_id(failure: &str, check_ids: &[String]) -> Option<String> {
    check_ids
        .iter()
        .find(|id| failure.contains(&format!(":{id}")) || failure.ends_with(id.as_str()))
        .cloned()
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
