use crate::red::fixture::row::{expected_check, expected_error};
use crate::red::fixtures::Observation;
use serde_json::Value;
use std::path::Path;

#[derive(Default)]
pub(crate) struct ObservationCache {
    namespace: crate::audit::namespace::law::ValueCache,
}

#[cfg(test)]
pub(crate) fn observation(
    root: &Path,
    expected: &Value,
    bad: &Value,
    base_path: &str,
) -> Option<Observation> {
    let target_digest = crate::package::inventory::package_digest(root).unwrap_or_default();
    observation_with_candidate(root, expected, bad, base_path, &target_digest)
}

#[cfg(test)]
pub(crate) fn observation_with_candidate(
    root: &Path,
    expected: &Value,
    bad: &Value,
    base_path: &str,
    target_digest: &str,
) -> Option<Observation> {
    let mut cache = ObservationCache::default();
    observation_with_candidate_cached(root, expected, bad, base_path, target_digest, &mut cache)
}

pub(crate) fn observation_with_candidate_cached(
    root: &Path,
    expected: &Value,
    bad: &Value,
    base_path: &str,
    target_digest: &str,
    cache: &mut ObservationCache,
) -> Option<Observation> {
    match base_path {
        "docs/source-cards.json" => Some(from_failures(
            expected,
            &crate::audit::text_guards::source_card_value_failures(bad),
        )),
        "docs/source-obligation-matrix.json" => {
            let mut failures = crate::audit::source_obligations::value_failures(bad);
            failures.extend(crate::audit::standards_gardening::value_failures(bad));
            failures.extend(crate::audit::text_guards::moving_value_value_failures(bad));
            failures.extend(crate::audit::text_guards::private_path_value_failures(bad));
            failures.extend(crate::audit::text_guards::stale_review_law_value_failures(
                bad,
            ));
            Some(from_failures(expected, &failures))
        }
        "docs/mandatory-law-surfaces.json" => Some(from_failures(
            expected,
            &crate::audit::mandatory::law::surfaces::value_failures(root, bad),
        )),
        "docs/foundational-law-traceability.json" => {
            let matrix =
                crate::json_boundary::read_json(&root.join("docs/source-obligation-matrix.json"))
                    .unwrap_or(serde_json::json!({}));
            Some(from_failures(
                expected,
                &crate::audit::foundational_law_trace::value_failures(root, &matrix, bad),
            ))
        }
        "templates/agent-standards/enforcement.json" => Some(from_failures(
            expected,
            &crate::audit::agent::standards::enforcement::value_failures(bad, Some(root)),
        )),
        "fixtures/agent-standards/valid/audit-pass-row.json" => Some(from_failures(
            expected,
            &crate::audit::agent::standards::tsv::checks::audit_row_failures(root, bad),
        )),
        "fixtures/template-integrity/valid/template-integrity.json" => Some(from_failures(
            expected,
            &crate::audit::template_integrity::value_failures(bad),
        )),
        "templates/.harness/coverage-manifest.json" => Some(from_failures(
            expected,
            &crate::audit::coverage::scope::value_failures_with_root(bad, Some(root)),
        )),
        "validation_artifacts/harness/fit-repo-receipt.json"
        | "templates/validation_artifacts/harness/fit-repo-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::plugin::product::cohesion::fit_repo_receipt_value_failures_with_candidate(
                root,
                bad,
                target_digest,
            ),
        )),
        ".codex-plugin/plugin.json" => Some(from_failures(
            expected,
            &crate::audit::plugin::product::cohesion::plugin_json_failures(bad),
        )),
        "plugin-manifest-draft.json" => Some(from_failures(
            expected,
            &crate::audit::namespace::law::value_failures_with_cache(
                root,
                bad,
                &mut cache.namespace,
            ),
        )),
        "docs/namespace-class-registry.json" => Some(from_failures(
            expected,
            &crate::audit::namespace::law::class_registry_value_failures(root, bad),
        )),
        "fixtures/law-surfaces/valid/runtime-tool-identity-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::law::surface::receipts::runtime_tool_identity_value_failures(root, bad),
        )),
        "fixtures/law-surfaces/valid/product-live-surface-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::law::surface::receipts::product_live_surface_value_failures(root, bad),
        )),
        "fixtures/law-surfaces/valid/transcript-quality-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::law::surface::receipts::transcript_quality_value_failures(root, bad),
        )),
        "fixtures/law-surfaces/valid/clean-checkout-command-discovery-receipt.json" => {
            Some(from_failures(
                expected,
                &crate::audit::law::surface::receipts::clean_checkout_value_failures(root, bad),
            ))
        }
        "fixtures/law-surfaces/valid/restartable-execplan-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::law::surface::receipts::restartable_execplan_value_failures(root, bad),
        )),
        "fixtures/law-surfaces/valid/memory-context-boundary-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::law::surface::receipts::memory_context_value_failures(root, bad),
        )),
        path if path.starts_with("fixtures/mandatory-law-surfaces/valid/") => Some(from_failures(
            expected,
            &crate::audit::mandatory::law::surfaces::receipt_value_failures_with_candidate(
                root,
                bad,
                target_digest,
            ),
        )),
        "docs/plugin-cohesion-manifest.json" => Some(from_failures(
            expected,
            &crate::audit::plugin::product::cohesion::flow_manifest_projection_failures(root, bad),
        )),
        "validation_artifacts/harness/plugin-product-journey-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::plugin::product::cohesion::journey_value_failures_with_candidate(
                root,
                bad,
                target_digest,
            ),
        )),
        "validation_artifacts/harness/product-fitness-receipt.json" => Some(from_failures(
            expected,
            &crate::audit::product::fitness::canonical_package_receipt_value_failures_with_candidate(
                root,
                bad,
                target_digest,
            ),
        )),
        path if path.starts_with("fixtures/review-materiality/valid/") => Some(from_failures(
            expected,
            &crate::review::materiality::value_failures(bad),
        )),
        "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json" => {
            Some(from_failures(
                expected,
                &crate::audit::standards_gardening::receipt_root_failures(root, bad),
            ))
        }
        _ => None,
    }
}

fn from_failures(expected: &Value, failures: &[String]) -> Observation {
    let expected_error = expected_error(expected);
    let ok = failures
        .iter()
        .any(|failure| failure.contains(&expected_error));
    Observation {
        check: expected_check(expected),
        ok,
        error: if ok {
            expected_error
        } else {
            failures
                .first()
                .cloned()
                .unwrap_or_else(|| "no_failure".to_string())
        },
    }
}
