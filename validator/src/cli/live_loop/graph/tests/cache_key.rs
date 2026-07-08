use crate::cli::live_loop::surfaces::surface_by_id;

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
