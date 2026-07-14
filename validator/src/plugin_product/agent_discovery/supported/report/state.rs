use super::model::{
    SupportedAgentAuthorityFinding, SupportedAgentAuthorityObservation,
    SupportedHostAgentAuthorityReport,
};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryErrorId;
use crate::plugin_product::agent_discovery::host::HostAgentAuthorityRequest;
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Default)]
pub(in super::super) struct ReportState {
    source_catalog_sha256: String,
    candidate_id: String,
    session_id: String,
    transaction_provenance_sha256: String,
    generation_sha256: Option<String>,
    observations: BTreeSet<SupportedAgentAuthorityObservation>,
    findings: BTreeSet<SupportedAgentAuthorityFinding>,
    capture_count: usize,
    effect_probe_count: usize,
    failure_code: Option<String>,
}

pub(in super::super) fn snapshot_report(
    report: &Arc<Mutex<ReportState>>,
) -> SupportedHostAgentAuthorityReport {
    let state = lock_report(report);
    SupportedHostAgentAuthorityReport {
        source_catalog_sha256: state.source_catalog_sha256.clone(),
        candidate_id: state.candidate_id.clone(),
        session_id: state.session_id.clone(),
        transaction_provenance_sha256: state.transaction_provenance_sha256.clone(),
        generation_sha256: state.generation_sha256.clone(),
        observations: state.observations.iter().cloned().collect(),
        findings: state.findings.iter().cloned().collect(),
        capture_count: state.capture_count,
        effect_probe_count: state.effect_probe_count,
        write_operation_count: 0,
        failure_code: state.failure_code.clone(),
    }
}

pub(in super::super) fn reset_report(
    report: &Arc<Mutex<ReportState>>,
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
) {
    *lock_report(report) = ReportState {
        source_catalog_sha256: source.catalog_sha256().to_owned(),
        candidate_id: request.candidate_id().to_owned(),
        session_id: request.session_id().to_owned(),
        transaction_provenance_sha256: request.provenance_sha256().to_owned(),
        ..ReportState::default()
    };
}

pub(in super::super) fn record_failure(
    report: &Arc<Mutex<ReportState>>,
    id: AgentDiscoveryErrorId,
) {
    let mut report = lock_report(report);
    if report.failure_code.is_none() {
        report.failure_code = Some(id.code().to_owned());
    }
}

pub(in super::super) fn set_generation(report: &Arc<Mutex<ReportState>>, generation: String) {
    lock_report(report).generation_sha256 = Some(generation);
}

pub(in super::super) fn increment_capture_count(report: &Arc<Mutex<ReportState>>) {
    lock_report(report).capture_count += 1;
}

pub(in super::super) fn increment_effect_probe_count(report: &Arc<Mutex<ReportState>>) {
    lock_report(report).effect_probe_count += 1;
}

pub(in super::super) fn lock_report(
    report: &Arc<Mutex<ReportState>>,
) -> MutexGuard<'_, ReportState> {
    report
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ReportState {
    pub(super) fn insert_observation(&mut self, row: SupportedAgentAuthorityObservation) {
        self.observations.insert(row);
    }

    pub(super) fn insert_finding(&mut self, row: SupportedAgentAuthorityFinding) {
        self.findings.insert(row);
    }
}
