use super::query::{BYTE_LIMIT, LiveQueryRoundtrip, RECEIPT_DIR, ROW_LIMIT};
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

const BACKEND_READINESS_TIMEOUT_MS: u64 = 500;

#[derive(Clone, Copy)]
pub(in crate::cli::live_loop::nodes::measurement::observation) struct LiveBackend {
    service: &'static str,
    endpoint_category: &'static str,
    health_url: &'static str,
    query: BackendQuery,
}

#[derive(Clone, Copy)]
enum BackendQuery {
    Logs,
    Metrics,
    Traces,
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn backend_for(
    roundtrip: LiveQueryRoundtrip,
) -> LiveBackend {
    match roundtrip {
        LiveQueryRoundtrip::Logs => LiveBackend {
            service: "victorialogs",
            endpoint_category: "logs_health",
            health_url: "http://127.0.0.1:9428/health",
            query: BackendQuery::Logs,
        },
        LiveQueryRoundtrip::Metrics => LiveBackend {
            service: "victoriametrics",
            endpoint_category: "metrics_health",
            health_url: "http://127.0.0.1:8428/health",
            query: BackendQuery::Metrics,
        },
        LiveQueryRoundtrip::Traces => LiveBackend {
            service: "victoriatraces",
            endpoint_category: "traces_health",
            health_url: "http://127.0.0.1:10428/health",
            query: BackendQuery::Traces,
        },
    }
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn ready(
    backend: LiveBackend,
) -> bool {
    let seconds = (BACKEND_READINESS_TIMEOUT_MS as f64 / 1000.0).to_string();
    Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            &seconds,
            backend.health_url,
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn write_unavailable_query_receipt(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: LiveQueryRoundtrip,
    backend: LiveBackend,
    receipt: &Path,
    run_id: &str,
    correlation_id: &str,
) -> Result<Value, String> {
    let failure_class = "live_loop_observability_backend_unavailable";
    let why_failed = format!(
        "{} backend health probe did not pass within {}ms",
        backend.service, BACKEND_READINESS_TIMEOUT_MS
    );
    let command = crate::cli::observe::types::ObserveCommand {
        operation: roundtrip.operation(),
        receipt: Some(receipt.to_path_buf()),
        query: Some(query_text(backend.query, run_id, correlation_id)),
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
    };
    let mut value = crate::cli::observe::telemetry::query_result(
        root,
        &command,
        query_kind(backend.query),
        command.query.clone().unwrap_or_default(),
        Vec::new(),
        "fail",
        Some(failure_class),
    )?;
    value["failure_class"] = json!(failure_class);
    value["observed_failure_class"] = json!(failure_class);
    value["why_failed"] = json!(why_failed);
    value["where_failed"] = json!(format!(
        "loop.measure.{}.observe.{}",
        surface.id,
        roundtrip.receipt_suffix()
    ));
    value["next_repair"] = json!(format!(
        "start or repair the {} live observability backend, rerun `target/debug/ultragoal --root . observe stack health`, then rerun `target/debug/ultragoal --root . loop measure --node {} --tier hot --cache-mode verified-local`",
        backend.service, surface.id
    ));
    value["claim_impact"] =
        json!("blocks_live_loop_speed_claim_until_same_candidate_live_query_reconciles");
    value["node_id"] = json!(surface.id);
    value["operation"] = json!(roundtrip.operation().id());
    value["backend_service"] = json!(backend.service);
    value["endpoint_category"] = json!(backend.endpoint_category);
    value["backend_readiness_timeout_ms"] = json!(BACKEND_READINESS_TIMEOUT_MS);
    let path =
        crate::output_path::claim_artifact_path(root, receipt, "live loop observe query receipt")?;
    crate::output_path::prepare_parent(&path)?;
    crate::json_boundary::write_json(&path, &value)?;
    Ok(value)
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn receipt_path(
    node_id: &str,
    suffix: &str,
) -> PathBuf {
    Path::new(RECEIPT_DIR).join(format!("{node_id}-{suffix}.json"))
}

fn query_kind(query: BackendQuery) -> &'static str {
    match query {
        BackendQuery::Logs => "logs",
        BackendQuery::Metrics => "metrics",
        BackendQuery::Traces => "traces",
    }
}

fn query_text(query: BackendQuery, run_id: &str, correlation_id: &str) -> String {
    match query {
        BackendQuery::Logs => format!("_time:5m run_id:{run_id} correlation_id:{correlation_id}"),
        BackendQuery::Metrics => crate::cli::observe::query::bounded_metric_query_for_operation(
            crate::cli::observe::types::ObserveOperation::MetricsQuery.id(),
        ),
        BackendQuery::Traces => {
            format!("{{run_id=\"{run_id}\", correlation_id=\"{correlation_id}\"}}")
        }
    }
}

#[cfg(test)]
#[path = "query_receipt_tests.rs"]
mod query_receipt_tests;
