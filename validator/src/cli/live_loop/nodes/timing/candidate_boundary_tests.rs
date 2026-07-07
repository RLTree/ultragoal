use super::{NODE_TIMING_REL, read_current};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[path = "test_rows.rs"]
mod test_rows;
use self::test_rows::{changed_inputs, current_timing_row, fmt_input};

#[test]
fn node_timing_reader_rejects_prior_candidate_boundary_rows() {
    let root = temp_root("live-loop-prior-boundary-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let surface =
        crate::cli::live_loop::surfaces::surface_by_id("coverage_full_script").expect("coverage");
    let input =
        crate::cli::live_loop::graph::surface_input_digest(surface, candidate, &changed, &context);
    let mut row = current_timing_row("sha256:prior", &input, "pass", "none");
    {
        let object = row.as_object_mut().expect("timing row object");
        object.insert("node_id".to_string(), json!(surface.id));
        object.insert(
            "canonical_full_command".to_string(),
            json!(surface.canonical_full_command),
        );
    }
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({ "nodes": [row] }))
        .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );

    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup prior boundary timing");
}

#[test]
fn node_timing_reader_rejects_proof_shaped_partial_or_failed_rows() {
    let root = temp_root("live-loop-invalid-status-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    let mut invalid_partial = current_timing_row(
        candidate,
        &input,
        "partial",
        "live_loop_telemetry_reconciliation_missing",
    );
    invalid_partial
        .as_object_mut()
        .expect("partial row")
        .insert("validation_status".to_string(), json!("fail"));
    let invalid_fail = current_timing_row(candidate, &input, "fail", "none");

    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({ "nodes": [invalid_partial, invalid_fail] }),
    )
    .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );

    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup invalid status timing");
}

fn context_digest() -> String {
    crate::digest::bytes(
        "validator=ultragoal-rust;law=observability-live-loop;tier=hot;cache=verified-local"
            .as_bytes(),
    )
}
