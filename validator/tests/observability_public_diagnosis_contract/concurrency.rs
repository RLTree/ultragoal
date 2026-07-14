use super::scenario::*;
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};
use ultragoal::observability::EventStore;

fn assert_lock_timeout(output: &std::process::Output, elapsed: Duration) {
    let timeout = Duration::from_millis(EventStore::supported_lock_timeout_millis());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert_eq!(value["exit_class"], "actionable_finding");
    assert!(elapsed >= timeout.saturating_sub(Duration::from_millis(50)));
    assert!(elapsed <= timeout + Duration::from_secs(2));
}

#[test]
fn contention_timeout_cancellation_and_release_are_bounded_zero_write() {
    let repository = Repository::new("lock-deadline", true, true);
    let binding = binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let mut retained = event(&binding, "lock-failure", 1, "check.run", "fail");
    retained.add_finding_ref(&finding.finding_id).unwrap();
    assert!(store.append(&retained).unwrap());
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(repository.store_path())
        .unwrap();
    holder.lock().unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let before = observe(repository.root());

    let started = Instant::now();
    let first = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_lock_timeout(&first, started.elapsed());
    assert_private_absent(&first, repository.root());
    assert_eq!(observe(repository.root()), before);
    let started = Instant::now();
    let second = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_lock_timeout(&second, started.elapsed());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(observe(repository.root()), before);

    let (_, diagnosis) = assert_payload_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &environment,
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "unavailable");
    assert_eq!(
        diagnosis["observability"]["read_failure"]["class"],
        "lock-timeout"
    );
    assert_eq!(diagnosis["observability"]["read_failure"]["stage"], "query");
    assert_eq!(diagnosis["observability"]["claim_effect"], "none");
    assert_no_connection(&listener);

    let mut command = repository.command_with_env(
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &environment,
    );
    let mut interrupted = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(250));
    assert!(interrupted.try_wait().unwrap().is_none());
    interrupted.kill().unwrap();
    interrupted.wait().unwrap();
    assert_eq!(observe(repository.root()), before);
    assert_no_connection(&listener);

    holder.unlock().unwrap();
    let (_, released_query) = assert_payload_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &environment,
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(released_query["events"][0]["event_id"], "lock-failure");
    let (_, released_diagnosis) = assert_payload_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &environment,
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(
        released_diagnosis["observability"]["matched_event_id"],
        "lock-failure"
    );
    assert_no_connection(&listener);
}
