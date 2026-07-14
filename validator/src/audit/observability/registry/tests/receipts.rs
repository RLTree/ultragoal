use serde_json::json;
use std::{fs, path::Path};

pub(super) fn write_command_roundtrip_receipts(root: &Path) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let dir = root.join("validation_artifacts/observability/command-roundtrip");
    fs::create_dir_all(&dir).expect("observability receipts");
    for command in super::super::command_inventory::REQUIRED_COMMANDS {
        write_receipt_set(
            &dir,
            &candidate,
            &slug(command),
            &format!("run-{}", slug(command)),
            &format!("corr-{}", slug(command)),
            &command_operation(command),
        );
    }
    for surface in super::super::surfaces::REQUIRED_SURFACES {
        let slug = slug(surface);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("surface-{slug}"),
            &format!("run-surface-{slug}"),
            &format!("corr-surface-{slug}"),
            &surface_operation(surface),
        );
    }
    for stage in super::super::operating::REQUIRED_LOOP_STAGES {
        let slug = slug(stage);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("loop-{slug}"),
            &format!("run-loop-{slug}"),
            &format!("corr-loop-{slug}"),
            &format!("observability.loop.{slug}"),
        );
    }
    for signal in super::super::operating::REQUIRED_SIGNAL_CLASSES {
        let slug = slug(signal);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("signal-{slug}"),
            &format!("run-signal-{slug}"),
            &format!("corr-signal-{slug}"),
            &format!("observability.signal.{slug}"),
        );
    }
    for family in super::super::dimension_ids::inventory_families() {
        for id in family.ids {
            let slug = slug(id);
            write_receipt_set(
                &dir,
                &candidate,
                &format!("{}-{slug}", family.board_key),
                &format!("run-{}-{slug}", family.board_key),
                &format!("corr-{}-{slug}", family.board_key),
                &format!("observability.{}.{}", family.board_key, slug),
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
    for kind in ["logs", "metrics", "traces"] {
        crate::json_boundary::write_json(
            &dir.join(format!("{slug}-{kind}.json")),
            &json!({
                "schema": crate::cli::observe::command::QUERY_SCHEMA,
                "status": "pass",
                "candidate_digest": candidate,
                "run_id": run,
                "correlation_id": corr,
                "query_kind": kind,
                "rows": [{
                    "candidate_digest": candidate,
                    "metric": {"__name__": "ultragoal_command_total"},
                    "operation": operation,
                    "correlation_id": corr
                }]
            }),
        )
        .expect("query receipt");
    }
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
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
