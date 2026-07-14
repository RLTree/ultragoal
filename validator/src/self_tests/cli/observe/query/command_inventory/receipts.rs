use serde_json::json;
use std::{fs, path::Path};

pub(super) fn write_command_roundtrip_receipts(root: &Path) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let dir = root.join("validation_artifacts/observability/command-roundtrip");
    fs::create_dir_all(&dir).expect("command roundtrip dir");
    write_command_receipts(&dir, &candidate);
    write_surface_receipts(&dir, &candidate);
    write_loop_receipts(&dir, &candidate);
    write_signal_receipts(&dir, &candidate);
    write_dimension_receipts(&dir, &candidate);
}

fn write_command_receipts(dir: &Path, candidate: &str) {
    for command in crate::audit::observability::required_commands() {
        let slug = slug(command);
        write_receipt_set(
            dir,
            candidate,
            &slug,
            &format!("run-{slug}"),
            &format!("corr-{slug}"),
            &command_operation(command),
        );
    }
}

fn write_surface_receipts(dir: &Path, candidate: &str) {
    for surface in crate::audit::observability::required_surfaces() {
        let slug = slug(surface);
        write_receipt_set(
            dir,
            candidate,
            &format!("surface-{slug}"),
            &format!("run-surface-{slug}"),
            &format!("corr-surface-{slug}"),
            &surface_operation(surface),
        );
    }
}

fn write_loop_receipts(dir: &Path, candidate: &str) {
    for stage in crate::audit::observability::required_loop_stages() {
        let slug = slug(stage);
        write_receipt_set(
            dir,
            candidate,
            &format!("loop-{slug}"),
            &format!("run-loop-{slug}"),
            &format!("corr-loop-{slug}"),
            &format!("observability.loop.{slug}"),
        );
    }
}

fn write_signal_receipts(dir: &Path, candidate: &str) {
    for signal in crate::audit::observability::required_signal_classes() {
        let slug = slug(signal);
        write_receipt_set(
            dir,
            candidate,
            &format!("signal-{slug}"),
            &format!("run-signal-{slug}"),
            &format!("corr-signal-{slug}"),
            &format!("observability.signal.{slug}"),
        );
    }
}

fn write_dimension_receipts(dir: &Path, candidate: &str) {
    for (board_key, _, _, ids) in crate::audit::observability::required_dimension_families() {
        for id in ids {
            let slug = slug(id);
            write_receipt_set(
                dir,
                candidate,
                &format!("{board_key}-{slug}"),
                &format!("run-{board_key}-{slug}"),
                &format!("corr-{board_key}-{slug}"),
                &format!("observability.{board_key}.{slug}"),
            );
        }
    }
}

fn write_receipt_set(
    dir: &Path,
    candidate: &str,
    slug: &str,
    run: &str,
    corr: &str,
    operation: &str,
) {
    crate::json_boundary::write_json(
        &dir.join(format!("{slug}.json")),
        &json!({
            "schema": crate::cli::observe::command::RECEIPT_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "operation": operation,
            "run_id": run,
            "correlation_id": corr
        }),
    )
    .expect("receipt");
    write_query_receipts(dir, candidate, slug, run, corr, operation);
}

fn write_query_receipts(
    dir: &Path,
    candidate: &str,
    slug: &str,
    run: &str,
    corr: &str,
    operation: &str,
) {
    for kind in ["logs", "metrics", "traces"] {
        let rows = if kind == "metrics" {
            json!([{
                "operation": operation,
                "metric": {"__name__": "ultragoal_command_total"}
            }])
        } else {
            json!([{
                "candidate_digest": candidate,
                "operation": operation,
                "correlation_id": corr
            }])
        };
        crate::json_boundary::write_json(
            &dir.join(format!("{slug}-{kind}.json")),
            &json!({
                "schema": crate::cli::observe::command::QUERY_SCHEMA,
                "status": "pass",
                "candidate_digest": candidate,
                "run_id": run,
                "correlation_id": corr,
                "query_kind": kind,
                "rows": rows
            }),
        )
        .expect("query receipt");
    }
}

fn slug(value: &str) -> String {
    value.replace(' ', "-")
}

fn command_operation(command: &str) -> String {
    match command {
        "line-cap check" => "line-caps.check".to_string(),
        "red fixture report" => "red_fixture.report".to_string(),
        "registry probe" => "registry_probe".to_string(),
        "update-goal eligibility" => "update_goal_eligibility".to_string(),
        "self update-goal eligibility" => "self_update_goal_eligibility".to_string(),
        _ => command.replace(' ', "."),
    }
}

fn surface_operation(surface: &str) -> String {
    format!("surface.{}", surface.replace(' ', "."))
}
