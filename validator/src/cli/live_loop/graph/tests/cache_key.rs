use crate::cli::live_loop::surfaces::surface_by_id;
use crate::scheduler::TaskClass;

#[test]
fn verified_local_cache_key_binds_the_narrow_command_contract() {
    let surface = surface_by_id("schema_validation").expect("schema validation surface");
    let old_contract = crate::cli::live_loop::surfaces::LoopValidationSurface {
        narrow_rerun: surface.canonical_full_command,
        ..surface
    };

    let current_key =
        super::super::verified_local_cache_key(surface, "sha256:input", "hot", "verified-local");
    let old_key = super::super::verified_local_cache_key(
        old_contract,
        "sha256:input",
        "hot",
        "verified-local",
    );

    assert_ne!(
        current_key, old_key,
        "verified-local rows must stale when the routine command changes"
    );
}

#[test]
fn verified_local_cache_key_binds_surface_spec_versions_and_cache_mode() {
    let surface = surface_by_id("schema_validation").expect("schema validation surface");

    let verified =
        super::super::verified_local_cache_key(surface, "sha256:input", "hot", "verified-local");
    let no_cache = super::super::verified_local_cache_key(surface, "sha256:input", "hot", "none");

    assert_ne!(
        verified, no_cache,
        "cache mode is part of the product-surface input spec authority"
    );
}

#[test]
fn missing_surface_input_spec_cannot_share_cache_key_with_specified_surface() {
    let specified = surface_by_id("schema_validation").expect("schema validation surface");
    let unspecified = crate::cli::live_loop::surfaces::LoopValidationSurface {
        id: "unknown_product_surface",
        surface: "unknown_product_surface",
        command: "unknown",
        canonical_full_command: "unknown",
        narrow_rerun: "unknown",
        telemetry_reconciliation_state: "unknown",
        execution_task_class: TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    };

    let specified_key =
        super::super::verified_local_cache_key(specified, "sha256:input", "hot", "verified-local");
    let unspecified_key = super::super::verified_local_cache_key(
        unspecified,
        "sha256:input",
        "hot",
        "verified-local",
    );

    assert_ne!(
        specified_key, unspecified_key,
        "missing input specs must fail closed instead of aliasing a broad fallback cache key"
    );
}
