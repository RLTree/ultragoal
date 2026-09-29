mod model;
mod recording;
mod state;

#[cfg(test)]
pub use model::SupportedAgentAuthorityFindingKind;
pub use model::SupportedHostAgentAuthorityReport;
pub(super) use recording::record_layer;
pub(super) use state::{
    ReportState, increment_capture_count, increment_effect_probe_count, record_failure,
    reset_report, set_generation, snapshot_report,
};
