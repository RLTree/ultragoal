use super::super::super::LiveLoopCommand;
use super::super::super::surfaces::{
    LOOP_VALIDATION_SURFACES, LoopValidationSurface, surface_by_id,
};

pub(super) fn selected_surfaces(
    command: &LiveLoopCommand,
) -> Result<Vec<LoopValidationSurface>, String> {
    if command.measure_all {
        return high_frequency_surfaces_from(LOOP_VALIDATION_SURFACES);
    }
    let node_id = command
        .node_id
        .as_deref()
        .ok_or_else(|| "loop measure requires --node <id> or --all".to_string())?;
    surface_by_id(node_id)
        .map(|surface| vec![surface])
        .ok_or_else(|| format!("unknown live-loop node: {node_id}"))
}

pub(super) fn high_frequency_surfaces_from(
    surfaces: &[LoopValidationSurface],
) -> Result<Vec<LoopValidationSurface>, String> {
    let selected: Vec<LoopValidationSurface> = surfaces
        .iter()
        .copied()
        .filter(|surface| surface.high_frequency)
        .collect();
    if selected.is_empty() {
        Err("live-loop high-frequency registry has no command surfaces".to_string())
    } else {
        Ok(selected)
    }
}
