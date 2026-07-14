use super::health as subject;
use serde_json::json;

#[test]
fn health_retry_only_accepts_running_starting_compose_rows() {
    let endpoint_pass = json!({
        "service": "victoriametrics",
        "check_source": "http_endpoint",
        "status": "pass"
    });
    let compose_starting = json!({
        "service": "victoriametrics",
        "check_source": "docker_compose_ps",
        "state": "running",
        "health": "starting",
        "status": "fail"
    });
    assert!(subject::should_retry_starting_health(&[
        endpoint_pass.clone(),
        compose_starting
    ]));

    let endpoint_fail = json!({
        "service": "victoriametrics",
        "check_source": "http_endpoint",
        "status": "fail"
    });
    assert!(!subject::should_retry_starting_health(&[endpoint_fail]));

    let compose_unhealthy = json!({
        "service": "victoriametrics",
        "check_source": "docker_compose_ps",
        "state": "running",
        "health": "unhealthy",
        "status": "fail"
    });
    assert!(!subject::should_retry_starting_health(&[
        endpoint_pass,
        compose_unhealthy
    ]));
}

#[test]
fn starting_compose_health_retry_is_bounded_by_command_timeout() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("stack-health-retry-timeout");
    std::fs::create_dir_all(&root).expect("root");
    let command = crate::cli::observe::command::ObserveCommand {
        operation: crate::cli::observe::command::ObserveOperation::StackHealth,
        receipt: None,
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 1024,
        timeout_ms: 1,
    };
    let rows = vec![json!({
        "service": "victoriametrics",
        "check_source": "docker_compose_ps",
        "state": "running",
        "health": "starting",
        "status": "fail"
    })];

    let retried = subject::wait_for_starting_compose_health(&root, &command, &[], rows);

    assert!(!retried.is_empty());
    assert!(
        retried
            .iter()
            .any(|row| row["check_source"] == "docker_compose_ps")
    );
    std::fs::remove_dir_all(root).expect("cleanup starting retry");
}
