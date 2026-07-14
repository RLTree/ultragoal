use crate::schema_catalog::SchemaStore;
use std::collections::BTreeMap;
use std::path::Path;

pub(in crate::audit) fn append_package_text_checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    append_base_checks(root, store, failures);
    let current = failures.clone();
    for failure in
        crate::audit::mandatory::law::surfaces::package_failures_with_current(root, &current)
    {
        if let Some(check_id) = mandatory_law_check_id(&failure, check_ids) {
            push(failures, &check_id, failure.clone());
        }
        push(failures, "source-obligation-coverage", failure);
    }
}

pub(in crate::audit) fn append_base_checks(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    source_and_standards(root, store, failures);
    provenance(root, store, failures);
    control_and_observability(root, failures);
    for failure in crate::audit::text_guards::private_home_path_failures(root) {
        push(failures, "plugin-inventory-closure", failure);
    }
}

fn source_and_standards(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    append(
        failures,
        "source-card-freshness",
        crate::audit::text_guards::source_card_freshness_failures(root),
    );
    append(
        failures,
        "source-obligation-coverage",
        crate::audit::source_obligations::failures(root),
    );
    append(
        failures,
        "source-obligation-coverage",
        crate::audit::law::surface::receipts::package_failures(root),
    );
    for (check, failure) in crate::audit::law::authority_surfaces::package_failures(root) {
        push(failures, &check, failure);
    }
    append(
        failures,
        "standards-gardener-promotion",
        crate::audit::standards_gardening::failures(root, store),
    );
    append(
        failures,
        "agent-standards-enforcement",
        crate::audit::agent::standards::enforcement::failures(root),
    );
    append(
        failures,
        "agent-standards-enforcement",
        crate::audit::template_integrity::package_failures(root),
    );
    append(
        failures,
        "agent-standards-enforcement",
        crate::audit::coverage::scope::package_failures(root),
    );
    append(
        failures,
        "agent-standards-enforcement",
        crate::audit::plugin::product::cohesion::package_failures(root),
    );
    append(
        failures,
        "product-fitness-proof",
        crate::audit::product::fitness::package_failures(root),
    );
}

fn provenance(root: &Path, store: &SchemaStore, failures: &mut BTreeMap<String, Vec<String>>) {
    append(
        failures,
        "validator-execution-provenance",
        crate::audit::text_guards::moving_value_drift_failures(root),
    );
    append(
        failures,
        "validator-execution-provenance",
        crate::audit::text_guards::stale_review_law_failures(root),
    );
    append(
        failures,
        "validator-execution-provenance",
        crate::audit::session_log_hardening::package_failures(root, store),
    );
    append(
        failures,
        "validator-execution-provenance",
        crate::audit::final_packet::claim_guard_failures(root, store),
    );
    append(
        failures,
        "validator-execution-provenance",
        crate::audit::plugin::laws::package_failures(root, store),
    );
}

fn control_and_observability(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    for failure in crate::audit::cli::control_plane::authority::package_failures(root) {
        push(failures, control_plane_check(&failure), failure);
    }
    append(
        failures,
        "cli-performance-latency-speed-iteration-fitness",
        crate::audit::cli::performance::package_failures(root),
    );
    for (check, failure) in crate::audit::rust::developer::package_failures(root) {
        push(failures, &check, failure);
    }
    append(
        failures,
        "full-local-observability-stack-integration-non-opaque-failure",
        crate::audit::observability::package_failures(root),
    );
    append(
        failures,
        "openai-api-key-model-cost-external-ai-boundary",
        crate::audit::openai::package_failures(root),
    );
    append(
        failures,
        "promptfoo-eval-red-team-provider-separation",
        crate::audit::promptfoo::package_failures(root),
    );
    append(
        failures,
        "halo-ranked-harness-change-optimization",
        crate::audit::halo::package_failures(root),
    );
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

fn append(failures: &mut BTreeMap<String, Vec<String>>, check: &str, details: Vec<String>) {
    for detail in details {
        push(failures, check, detail);
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
