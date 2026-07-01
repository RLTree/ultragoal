#[derive(Clone, Copy)]
pub(crate) struct InventoryFamily {
    pub(crate) board_key: &'static str,
    pub(crate) list_key: &'static str,
    pub(crate) inventory_key: &'static str,
    pub(crate) failure_prefix: &'static str,
    pub(crate) ids: &'static [&'static str],
}

const VALIDATOR_CHECK_FAMILIES: &[&str] = &[
    "standards enforcement",
    "source obligations",
    "foundational trace",
    "schema catalog",
    "mandatory-law surfaces",
    "package inventory",
    "namespace",
    "line caps",
    "coverage",
    "product proofs",
    "review and packet proofs",
    "red fixture evaluation",
    "target repo audit",
    "research trace",
    "observability registry",
    "claim guards",
];

const RECEIPT_PROOF_PATHS: &[&str] = &[
    "package digest receipt",
    "source audit receipt",
    "red fixture report receipt",
    "coverage receipt",
    "product fitness receipt",
    "product journey receipt",
    "fit repo receipt",
    "standards gardener receipt",
    "review target receipt",
    "archive receipt",
    "final packet proof",
    "self law receipt",
    "update goal eligibility receipt",
    "install audit receipt",
    "cache audit receipt",
    "registry exposure receipt",
    "openai call receipt",
    "promptfoo receipt",
    "halo capability receipt",
    "rust proof receipts",
    "gc receipts",
];

const FIXTURE_REPORT_PATHS: &[&str] = &[
    "red fixtures catalog",
    "red fixture report",
    "mandatory law valid fixtures",
    "green fixture paths",
    "tamper fixture paths",
    "schema fixtures",
    "review round fixtures",
    "target repo valid fixtures",
    "target repo red fixtures",
];

const PACKAGE_SETUP_SURFACES: &[&str] = &[
    "plugin manifest draft",
    "codex plugin manifest",
    "skills",
    "templates",
    "schemas",
    "observability docs",
    "package resource map",
    "agent first repo init",
    "agent first repo retrofit",
    "fit repo setup",
    "target repo audit setup",
    "clean checkout command discovery",
];

const LONG_RUNNING_PATHS: &[&str] = &[
    "source audit",
    "red fixture report",
    "coverage prove",
    "install audit",
    "cache audit",
    "registry probe",
    "review target build",
    "archive build",
    "rust release",
    "rust coverage prove",
    "rust watch",
    "gc apply",
    "observe stack up",
    "observe stack smoke",
    "openai call prove",
    "promptfoo prove",
    "halo capability prove",
];

const EXTERNAL_LIVE_PATHS: &[&str] = &[
    "openai provider calls",
    "promptfoo execution",
    "halo adapter",
    "install surface",
    "cache surface",
    "app registry surface",
    "reviewer surface",
    "observability live stack",
    "trace exporter",
    "metrics exporter",
    "logs exporter",
    "target repo runtime",
];

const CLAIM_GUARDS: &[&str] = &[
    "completion",
    "readiness",
    "release",
    "final packet correctness",
    "update goal eligibility",
    "reviewer exposure",
    "app registry exposure",
    "install parity",
    "cache parity",
    "product success",
    "observability gate",
    "local spool only proof",
    "live stack query proof",
];

pub(crate) fn inventory_families() -> Vec<InventoryFamily> {
    vec![
        family(
            "validator_checks",
            "validator_check_families",
            "validator_check_inventory",
            "observability_validator_check",
            VALIDATOR_CHECK_FAMILIES,
        ),
        family(
            "receipts",
            "receipt_proof_paths",
            "receipt_proof_inventory",
            "observability_receipt_proof",
            RECEIPT_PROOF_PATHS,
        ),
        family(
            "fixtures",
            "fixture_report_paths",
            "fixture_report_inventory",
            "observability_fixture_report",
            FIXTURE_REPORT_PATHS,
        ),
        family(
            "package_setup",
            "package_plugin_setup_retrofit_surfaces",
            "package_plugin_setup_retrofit_inventory",
            "observability_package_setup",
            PACKAGE_SETUP_SURFACES,
        ),
        family(
            "long_running",
            "long_running_paths",
            "long_running_path_inventory",
            "observability_long_running",
            LONG_RUNNING_PATHS,
        ),
        family(
            "external_live",
            "external_live_paths",
            "external_live_path_inventory",
            "observability_external_live",
            EXTERNAL_LIVE_PATHS,
        ),
        family(
            "claim_guards",
            "claim_guards",
            "claim_guard_inventory",
            "observability_claim_guard",
            CLAIM_GUARDS,
        ),
    ]
}

fn family(
    board_key: &'static str,
    list_key: &'static str,
    inventory_key: &'static str,
    failure_prefix: &'static str,
    ids: &'static [&'static str],
) -> InventoryFamily {
    InventoryFamily {
        board_key,
        list_key,
        inventory_key,
        failure_prefix,
        ids,
    }
}
