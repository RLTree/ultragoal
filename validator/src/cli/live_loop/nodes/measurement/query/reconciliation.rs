use crate::cli::live_loop::surfaces::LoopValidationSurface;
use std::path::Path;
use std::time::Instant;

#[path = "roundtrip.rs"]
mod roundtrip;
pub(in crate::cli::live_loop::nodes::measurement::observation) use roundtrip::{
    BYTE_LIMIT, BackendState, LiveQueryRoundtrip, RECEIPT_DIR, ROW_LIMIT, RoundtripQuery,
};
#[cfg(test)]
pub(in crate::cli::live_loop::nodes::measurement::observation) use roundtrip::{
    EXPLAIN_TIMEOUT_MS, LIVE_BACKEND_QUERY_TIMEOUT_MS, TRACE_QUERY_TIMEOUT_MS,
};
pub(in crate::cli::live_loop::nodes::measurement::observation) use super::receipt_capture::{
    PendingQueryReceipt, capture_live_query, write_pending_query_receipt,
};
#[cfg(test)]
pub(in crate::cli::live_loop::nodes::measurement::observation) use super::receipt_capture::{
    run_observe_query, unavailable_backend_receipt,
};
pub(in crate::cli::live_loop::nodes::measurement::observation) use super::super::observe_receipt_reader::{
    ObserveReceipt, receipt_path,
};

#[cfg(test)]
pub(in crate::cli::live_loop::nodes::measurement::observation) use super::super::observe_receipt_reader::{
    read_observe_receipt, receipt_status,
};

pub(in crate::cli::live_loop::nodes::measurement::observation) fn run(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: RoundtripQuery,
    run_id: &str,
    correlation_id: &str,
    candidate: &str,
) -> Result<ObserveReceipt, String> {
    let live_state = live_query_roundtrip(roundtrip)
        .map(|live_roundtrip| (live_roundtrip, live_backend_state(live_roundtrip)));
    run_with_observation_state(
        root,
        surface,
        roundtrip,
        run_id,
        correlation_id,
        candidate,
        live_state,
    )
}

fn run_with_observation_state(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: RoundtripQuery,
    run_id: &str,
    correlation_id: &str,
    candidate: &str,
    live_state: Option<(LiveQueryRoundtrip, BackendState)>,
) -> Result<ObserveReceipt, String> {
    let started = Instant::now();
    let receipt = receipt_path(surface.id, roundtrip.receipt_suffix());
    let mut observe_receipt = match live_state {
        Some((live_roundtrip, state)) => run_with_backend_state(
            root,
            surface,
            live_roundtrip,
            receipt,
            run_id,
            correlation_id,
            state,
            candidate,
        )?,
        None => super::receipt_capture::run_observe_query(
            root,
            roundtrip,
            receipt,
            run_id,
            correlation_id,
        )?,
    };
    observe_receipt.duration_ms = elapsed_ms(started);
    Ok(observe_receipt)
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn capture_query_roundtrip(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: LiveQueryRoundtrip,
    run_id: &str,
    correlation_id: &str,
    candidate: &str,
) -> PendingQueryReceipt {
    let receipt = receipt_path(surface.id, roundtrip.receipt_suffix());
    let state = live_backend_state(roundtrip);
    capture_live_query(
        root,
        surface,
        roundtrip,
        receipt,
        run_id,
        correlation_id,
        state,
        candidate,
    )
}

fn run_with_backend_state(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: LiveQueryRoundtrip,
    receipt: std::path::PathBuf,
    run_id: &str,
    correlation_id: &str,
    state: BackendState,
    candidate: &str,
) -> Result<ObserveReceipt, String> {
    let pending = capture_live_query(
        root,
        surface,
        roundtrip,
        receipt,
        run_id,
        correlation_id,
        state,
        candidate,
    );
    write_pending_query_receipt(root, pending)
}

fn live_backend_state(roundtrip: LiveQueryRoundtrip) -> BackendState {
    let backend = super::super::live_backend::backend_for(roundtrip);
    backend_state_from_probe(backend, super::super::live_backend::ready(backend))
}

fn live_query_roundtrip(roundtrip: RoundtripQuery) -> Option<LiveQueryRoundtrip> {
    match roundtrip {
        RoundtripQuery::Logs => Some(LiveQueryRoundtrip::Logs),
        RoundtripQuery::Metrics => Some(LiveQueryRoundtrip::Metrics),
        RoundtripQuery::Traces => Some(LiveQueryRoundtrip::Traces),
        RoundtripQuery::ExplainFailure => None,
    }
}

fn backend_state_from_probe(
    backend: super::super::live_backend::LiveBackend,
    ready: bool,
) -> BackendState {
    match ready {
        true => BackendState::Ready,
        false => BackendState::Unavailable(backend),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[cfg(test)]
#[path = "../observation/probe_result_tests.rs"]
mod backend_probe_result_tests;
#[cfg(test)]
#[path = "../observation/artifact_publication_tests.rs"]
mod receipt_tests;
#[cfg(test)]
#[path = "../observation/availability_query_tests.rs"]
mod tests;
