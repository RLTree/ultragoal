use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::cli::observe::command::ObserveCommand;
use crate::cli::observe::query::LiveQueryObservation;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::super::live_backend;
use super::super::observe_receipt_reader::{ObserveReceipt, read_observe_receipt, receipt_status};
use super::reconciliation::{
    BYTE_LIMIT, BackendState, LiveQueryRoundtrip, ROW_LIMIT, RoundtripQuery,
};

pub(in crate::cli::live_loop::nodes::measurement::observation) enum PendingQueryReceipt {
    Live(LiveQueryReceipt),
    Unavailable {
        surface: LoopValidationSurface,
        roundtrip: LiveQueryRoundtrip,
        receipt: PathBuf,
        run_id: String,
        correlation_id: String,
        backend: live_backend::LiveBackend,
    },
}

pub(in crate::cli::live_loop::nodes::measurement::observation) struct LiveQueryReceipt {
    pub(in crate::cli::live_loop::nodes::measurement::observation) roundtrip: LiveQueryRoundtrip,
    pub(in crate::cli::live_loop::nodes::measurement::observation) receipt: PathBuf,
    pub(in crate::cli::live_loop::nodes::measurement::observation) command: ObserveCommand,
    pub(in crate::cli::live_loop::nodes::measurement::observation) observation:
        LiveQueryObservation,
    pub(in crate::cli::live_loop::nodes::measurement::observation) query_duration_ms: u64,
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn capture_live_query(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: LiveQueryRoundtrip,
    receipt: PathBuf,
    run_id: &str,
    correlation_id: &str,
    state: BackendState,
    candidate: &str,
) -> PendingQueryReceipt {
    if let BackendState::Unavailable(backend) = state {
        return PendingQueryReceipt::Unavailable {
            surface,
            roundtrip,
            receipt,
            run_id: run_id.to_string(),
            correlation_id: correlation_id.to_string(),
            backend,
        };
    }
    let query_kind = roundtrip.query_kind();
    let started = Instant::now();
    let command = observe_command(roundtrip.into(), receipt.clone(), run_id, correlation_id);
    let observation = crate::cli::observe::query::capture_for_candidate(
        root,
        &command,
        query_kind,
        candidate.to_string(),
    );
    PendingQueryReceipt::Live(LiveQueryReceipt {
        roundtrip,
        receipt,
        command,
        observation,
        query_duration_ms: elapsed_ms(started),
    })
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn write_pending_query_receipt(
    root: &Path,
    pending: PendingQueryReceipt,
) -> Result<ObserveReceipt, String> {
    match pending {
        PendingQueryReceipt::Live(live) => write_live_query_receipt(root, live),
        PendingQueryReceipt::Unavailable {
            surface,
            roundtrip,
            receipt,
            run_id,
            correlation_id,
            backend,
        } => {
            let mut receipt = unavailable_backend_receipt(
                root,
                surface,
                roundtrip,
                &receipt,
                &run_id,
                &correlation_id,
                backend,
            )?;
            receipt.duration_ms = receipt.duration_ms.max(1);
            Ok(receipt)
        }
    }
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn run_observe_query(
    root: &Path,
    roundtrip: RoundtripQuery,
    receipt: PathBuf,
    run_id: &str,
    correlation_id: &str,
) -> Result<ObserveReceipt, String> {
    let command = observe_command(roundtrip, receipt.clone(), run_id, correlation_id);
    let code = crate::cli::observe::run(root, &command)?;
    read_observe_receipt(root, &receipt, roundtrip, code)
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn unavailable_backend_receipt(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: LiveQueryRoundtrip,
    receipt: &Path,
    run_id: &str,
    correlation_id: &str,
    backend: live_backend::LiveBackend,
) -> Result<ObserveReceipt, String> {
    let value = live_backend::write_unavailable_query_receipt(
        root,
        surface,
        roundtrip.into(),
        backend,
        receipt,
        run_id,
        correlation_id,
    )?;
    Ok(ObserveReceipt {
        receipt: receipt.display().to_string(),
        exit_code: 1,
        status: "fail".to_string(),
        duration_ms: 1,
        value,
    })
}

fn write_live_query_receipt(root: &Path, live: LiveQueryReceipt) -> Result<ObserveReceipt, String> {
    let started = Instant::now();
    let query_kind = live.roundtrip.query_kind();
    let value = crate::cli::observe::query::receipt_from_observation(
        root,
        &live.command,
        query_kind,
        live.observation,
    )?;
    let absolute = crate::output_path::claim_artifact_path(
        root,
        &live.receipt,
        "live loop observe query receipt",
    )?;
    crate::json_boundary::write_json(&absolute, &value)?;
    let status = receipt_status(&value, live.roundtrip.into())?.to_string();
    let exit_code = i32::from(status != "pass");
    Ok(ObserveReceipt {
        receipt: live.receipt.display().to_string(),
        exit_code,
        status,
        duration_ms: live.query_duration_ms.saturating_add(elapsed_ms(started)),
        value,
    })
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn observe_command(
    roundtrip: RoundtripQuery,
    receipt: PathBuf,
    run_id: &str,
    correlation_id: &str,
) -> ObserveCommand {
    ObserveCommand {
        operation: roundtrip.operation(),
        receipt: Some(receipt),
        query: None,
        run_id: Some(run_id.to_string()),
        correlation_id: Some(correlation_id.to_string()),
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: ROW_LIMIT,
        byte_limit: BYTE_LIMIT,
        timeout_ms: roundtrip.timeout_ms(),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
