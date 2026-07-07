use super::super::super::{LiveLoopAction, LiveLoopCommand};
use std::path::PathBuf;

fn command_all(receipt: PathBuf) -> LiveLoopCommand {
    LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt,
        node_id: None,
        measure_all: true,
    }
}

#[test]
fn live_loop_measure_all_selects_high_frequency_nodes_only() {
    let command =
        command_all("validation_artifacts/observability/live-loop-node-timing.json".into());
    let surfaces = super::surface_selection::selected_surfaces(&command).expect("surfaces");
    assert!(surfaces.iter().any(|surface| surface.id == "fmt_check"));
    assert!(surfaces.iter().any(|surface| surface.id == "build_check"));
    assert!(!surfaces.iter().any(|surface| surface.id == "changed_files"));
}

#[test]
fn live_loop_measure_all_fails_closed_for_empty_high_frequency_registry() {
    let error = super::surface_selection::high_frequency_surfaces_from(&[])
        .err()
        .unwrap_or_default();
    assert!(
        error.contains("high-frequency registry has no command surfaces"),
        "{error}"
    );
}
