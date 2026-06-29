use crate::json_boundary;
use crate::schema_catalog::{self, SchemaStore};
use crate::target_fixtures;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub fn checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    validator_artifacts: &[Value],
) -> BTreeMap<String, Vec<String>> {
    let mut failures = check_ids
        .iter()
        .map(|id| (id.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    mapped_schema_checks(root, store, &mut failures);
    inventory_checks(root, &mut failures);
    crate::audit::red::catalog::check(root, store, &mut failures);
    text_guard_checks(root, store, check_ids, &mut failures);
    crate::audit::review_history::check(root, &mut failures);
    for failure in crate::review::round::fixture_failures(root) {
        push(&mut failures, "validator-execution-provenance", failure);
    }
    for failure in crate::review::materiality::fixture_failures(root) {
        push(&mut failures, "material-review-scope-gate", failure);
    }
    failures
        .entry("target-repo-audit-capability".to_string())
        .or_default()
        .extend(target_fixtures::target_capability_failures(
            root,
            validator_artifacts,
        ));
    failures
}

fn text_guard_checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
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
    for failure in crate::audit::cli::control_plane::authority::package_failures(root) {
        let check = if failure.contains("self-law")
            || failure.contains("self_law")
            || failure.contains("cli-self-law-compliance")
        {
            "cli-self-law-compliance"
        } else {
            "cli-control-plane-authority"
        };
        push(failures, check, failure);
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
    for failure in crate::audit::text_guards::private_home_path_failures(root) {
        push(failures, "plugin-inventory-closure", failure);
    }
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

pub fn final_hygiene_check(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    let bytecode = crate::package::inventory::final_bytecode_failures(root);
    if !bytecode.is_empty() {
        failures
            .entry("plugin-inventory-closure".to_string())
            .or_default()
            .extend(bytecode);
    }
}

fn mapped_schema_checks(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let mut mapped = crate::audit::package::schema::map::base();
    add_mapped_globs(root, &mut mapped);
    for (rel, schema) in mapped {
        match json_boundary::read_json(&root.join(&rel)) {
            Ok(value) => {
                if let Some(first) = schema_catalog::schema_errors(store, schema, &value).first() {
                    push(failures, "schema-valid", format!("{rel}: {first}"));
                }
            }
            Err(err) => push(
                failures,
                "schema-valid",
                format!("{rel}: json_load_failed: {err}"),
            ),
        }
    }
}

fn mandatory_law_check_id(failure: &str, check_ids: &[String]) -> Option<String> {
    check_ids
        .iter()
        .find(|id| failure.contains(&format!(":{id}")) || failure.ends_with(id.as_str()))
        .cloned()
}

fn add_mapped_globs(root: &Path, mapped: &mut BTreeMap<String, &'static str>) {
    add_glob(root, "fixtures/valid", "fixture-bundle.schema.json", mapped);
    add_glob(root, "fixtures/red", "red-packet.schema.json", mapped);
    add_glob(
        root,
        "fixtures/review-round/valid",
        "review-round-receipt.schema.json",
        mapped,
    );
    add_glob(
        root,
        "fixtures/review-materiality/valid",
        "review-materiality-gate.schema.json",
        mapped,
    );
    add_glob(
        root,
        "fixtures/mandatory-law-surfaces/valid",
        "mandatory-law-surface-receipt.schema.json",
        mapped,
    );
}

fn add_glob(
    root: &Path,
    dir: &str,
    schema: &'static str,
    mapped: &mut BTreeMap<String, &'static str>,
) {
    if let Ok(entries) = std::fs::read_dir(root.join(dir)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Ok(rel) = path.strip_prefix(root)
            {
                mapped.insert(rel.to_string_lossy().replace('\\', "/"), schema);
            }
        }
    }
}

fn inventory_checks(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    match json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
        Ok(manifest) => {
            for failure in crate::package::inventory::inventory_closure_failures(root, &manifest) {
                let check = if failure.contains("duplicates=") {
                    "plugin-inventory-exactly-once"
                } else {
                    "plugin-inventory-closure"
                };
                push(failures, check, failure);
            }
            for failure in crate::skill_links::manifest_failures(root, &manifest) {
                push(failures, "skill-inventory-closure", failure.detail);
            }
            for failure in crate::package::resource::purpose::failures(root, &manifest) {
                push(
                    failures,
                    "plugin-inventory-closure",
                    format!("{}: {}", failure.code, failure.detail),
                );
            }
            for failure in crate::audit::namespace::law::package_failures(root, &manifest) {
                push(failures, "namespace-progressive-disclosure", failure);
            }
        }
        Err(err) => push(
            failures,
            "plugin-inventory-closure",
            format!("plugin-manifest-draft.json load failed: {err}"),
        ),
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
