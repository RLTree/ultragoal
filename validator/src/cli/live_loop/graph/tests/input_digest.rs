#[test]
fn high_frequency_input_digest_uses_affected_inputs_not_whole_candidate() {
    let fmt_surface =
        super::super::super::surfaces::surface_by_id("fmt_check").expect("fmt check surface");
    let changed = crate::digest::bytes(b"validator/src/cli/live_loop/mod.rs");
    let context = crate::digest::bytes(b"validator-law-schema-fixture-versions");
    let current =
        super::super::surface_input_digest(fmt_surface, "sha256:current", &changed, &context);
    let unrelated_candidate =
        super::super::surface_input_digest(fmt_surface, "sha256:other", &changed, &context);
    assert_eq!(
        current, unrelated_candidate,
        "hot-loop cache equivalence must be affected-input local, not whole-candidate local"
    );

    let coverage_surface = super::super::super::surfaces::surface_by_id("coverage_full_script")
        .expect("coverage boundary surface");
    let coverage_current =
        super::super::surface_input_digest(coverage_surface, "sha256:current", &changed, &context);
    let coverage_other =
        super::super::surface_input_digest(coverage_surface, "sha256:other", &changed, &context);
    assert_ne!(
        coverage_current, coverage_other,
        "strict boundary proof remains candidate-bound"
    );
}
