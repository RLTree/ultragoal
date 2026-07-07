use super::*;
use crate::cli::live_loop::nodes::measurement::observation::live_backend;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn run_reports_unavailable_backend_receipt_write_failure_without_erasing_query_context() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-run-unavailable-write-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = surface_by_id("changed_files").expect("surface");
    let backend = live_backend::backend_for(LiveQueryRoundtrip::Logs);

    let err = run_with_observation_state(
        &root,
        surface,
        RoundtripQuery::Logs,
        "run-unavailable-write-failure",
        "corr-unavailable-write-failure",
        "sha256:missing-package-candidate",
        Some((LiveQueryRoundtrip::Logs, BackendState::Unavailable(backend))),
    )
    .expect_err("missing package manifest blocks unavailable backend receipt");

    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    assert!(
        !root.join(receipt_path(surface.id, "logs-query")).exists(),
        "receipt publication must fail closed when package authority is absent"
    );
    std::fs::remove_dir_all(root).expect("cleanup unavailable run write failure");
}

#[test]
fn unavailable_backend_receipt_requires_concrete_backend_authority() {
    let root =
        super::tests::prepared_root("live-loop-query-roundtrip-concrete-unavailable-backend");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend = live_backend::backend_for(LiveQueryRoundtrip::Logs);

    let receipt = unavailable_backend_receipt(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        &receipt_path,
        "run-concrete-unavailable",
        "corr-concrete-unavailable",
        backend,
    )
    .expect("concrete unavailable backend receipt");

    assert_eq!(receipt.exit_code, 1);
    assert_eq!(receipt.status, "fail");
    assert_eq!(receipt.value["backend_service"], "victorialogs");
    assert!(root.join(receipt_path).exists());
    std::fs::remove_dir_all(root).expect("cleanup concrete unavailable backend");
}

#[test]
fn backend_probe_result_maps_to_concrete_backend_state() {
    let backend = live_backend::backend_for(LiveQueryRoundtrip::Logs);

    assert!(matches!(
        backend_state_from_probe(backend, true),
        BackendState::Ready
    ));
    assert!(matches!(
        backend_state_from_probe(backend, false),
        BackendState::Unavailable(_)
    ));
}
