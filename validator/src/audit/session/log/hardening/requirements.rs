pub(super) const REQUIRED_CLASSES: &[&str] = &[
    "stale_review_round_assumptions",
    "active_registry_proof_overclaim",
    "disk_installed_cache_substituted_for_app_registry",
    "reviewer_ready_without_current_exposure",
    "product_fitness_docs_only",
    "product_fitness_substitutions",
    "product::fitness::substitutions",
    "source_install_cache_drift",
    "stale_validator_receipts_or_red_fixtures",
    "package_inventory_holes",
    "packet_without_actionable_findings",
    "standards_rows_prose_only",
    "previous_followup_blocker_stale_missing_overclaim_terms",
    "typed_boundary_self_audit",
    "coverage_100_self_audit",
    "line_cap_self_audit",
    "product_fitness_review_ownership",
    "namespace_progressive_disclosure_under_enforced",
];

pub(super) const REQUIRED_SOURCE_IDS: &[&str] = &[
    "chronicle-product-fitness-0449",
    "chronicle-source-install-cache-0650",
    "chronicle-registry-proof-1706",
    "chronicle-packet-gap-1847",
    "chronicle-packet-completion-1906",
    "chronicle-product-fitness-ownership-2231",
    "chronicle-product-fitness-ownership-2232",
    "chronicle-namespace-progressive-disclosure-2236",
    "session-installed-drift-0907",
    "session-packet-run-1857",
    "session-self-audit-1935",
    "packet-summary-1900",
];

pub(super) const REQUIRED_FINDING_STRINGS: &[&str] = &[
    "signal",
    "observed_bad_behavior",
    "implementation_status",
    "evidence_digest",
];

pub(super) const REQUIRED_FINDING_ARRAYS: &[&str] = &[
    "affected_law_ids",
    "affected_package_surfaces",
    "fixture_ids",
    "validator_ids",
    "receipt_ids",
    "claim_ids",
];
