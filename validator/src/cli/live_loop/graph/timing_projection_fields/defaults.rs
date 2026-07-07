use super::super::super::surfaces::{BOUNDARY_PROOF_POLICY, LoopValidationSurface};

pub(super) fn text<'a>(
    surface: LoopValidationSurface,
    timing: Option<&'a super::super::super::nodes::timing::NodeTiming>,
    field: impl FnOnce(&'a super::super::super::nodes::timing::NodeTiming) -> &'a str,
) -> &'a str {
    timing
        .map(field)
        .unwrap_or_else(|| projection_text(surface))
}

pub(super) fn projection_text(surface: LoopValidationSurface) -> &'static str {
    if surface.high_frequency {
        "missing"
    } else if surface.hot_loop_policy == BOUNDARY_PROOF_POLICY {
        "withheld_until_boundary"
    } else {
        "observation_only"
    }
}

pub(super) fn affected_set_status(surface: LoopValidationSurface) -> &'static str {
    if surface.high_frequency {
        "missing_current_timing_record"
    } else if surface.hot_loop_policy == BOUNDARY_PROOF_POLICY {
        "boundary_proof_withheld_from_hot_loop"
    } else {
        "context_observation_not_validation_work"
    }
}
