mod model;
mod recording;
mod state;

pub use model::SupportedHostAgentAuthorityReport;
#[cfg(test)]
pub use model::{
    SupportedAgentAuthorityFinding, SupportedAgentAuthorityFindingKind,
    SupportedAgentAuthorityObservation,
};
pub(super) use recording::record_layer;
#[cfg(test)]
pub(super) use state::lock_report;
pub(super) use state::{
    ReportState, increment_capture_count, increment_effect_probe_count, record_failure,
    reset_report, set_generation, snapshot_report,
};
