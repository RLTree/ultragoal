use super::*;

#[test]
pub(crate) fn bound_race_symlink_and_fifo_substitution_refuse_without_hidden_effects() {
    let raced = JourneyRepository::new("bound-race", false, false);
    let binding = public_binding(&raced);
    let store = open_store(&raced, &binding);
    assert!(
        store
            .append(&event(&binding, "race-event", 1, "check.run"))
            .unwrap()
    );
    let original = raced
        .store_path()
        .with_file_name("successor-events.original.jsonl");
    fs::rename(raced.store_path(), &original).unwrap();
    fs::write(raced.store_path(), b"").unwrap();
    let before_race_query = observe(raced.root());
    let error = store.query(&query(&binding)).unwrap_err();
    assert!(
        error.contains("identity")
            || error.contains("changed")
            || error.contains("replaced")
            || error.contains("substitution"),
        "unexpected race error: {error}"
    );
    assert_eq!(observe(raced.root()), before_race_query);
    assert_eq!(fs::read(raced.store_path()).unwrap(), b"");
    assert!(!fs::read(&original).unwrap().is_empty());

    let linked = JourneyRepository::new("symlink-parent", false, false);
    let outside = linked.outside("outside-spool");
    fs::create_dir_all(linked.root().join("validation_artifacts/observability")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(
        &outside,
        linked
            .root()
            .join("validation_artifacts/observability/spool"),
    )
    .unwrap();
    let outside_before = fs::read_dir(&outside).unwrap().count();
    assert_diagnostic_zero_write(
        &linked,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), outside_before);

    let fifo = JourneyRepository::new("fifo-leaf", false, false);
    fs::create_dir_all(fifo.store_path().parent().unwrap()).unwrap();
    let fifo_name = CString::new(fifo.store_path().as_os_str().as_bytes()).unwrap();
    let result = unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) };
    assert_eq!(
        result,
        0,
        "mkfifo failed: {}",
        std::io::Error::last_os_error()
    );
    assert_diagnostic_zero_write(
        &fifo,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    raced.teardown();
    linked.teardown();
    fifo.teardown();
}

#[test]
pub(crate) fn source_built_query_exits_at_the_lock_deadline_with_deterministic_zero_write_output() {
    let repository = JourneyRepository::new("exclusive-lock-deadline", true, true);
    let binding = public_binding(&repository);
    let store = open_store(&repository, &binding);
    assert!(
        store
            .append(&event(&binding, "lock-deadline-event", 1, "check.run"))
            .unwrap()
    );
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(repository.store_path())
        .unwrap();
    holder.lock().unwrap();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind network canary");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let before = observe(repository.root());
    let timeout = Duration::from_millis(EventStore::supported_lock_timeout_millis());

    let first_started = Instant::now();
    let first = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_public_lock_timeout(&first, repository.root(), first_started.elapsed(), timeout);
    assert_eq!(
        observe(repository.root()),
        before,
        "contended query wrote state"
    );
    assert_no_connection(&listener);

    let mut interrupted_command =
        repository.command_with_env(&["--json", "observe", "query"], &environment);
    let mut interrupted = interrupted_command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn interruptible source-built query");
    thread::sleep(Duration::from_millis(250));
    assert!(
        interrupted.try_wait().unwrap().is_none(),
        "contended query did not remain in its bounded wait"
    );
    interrupted.kill().unwrap();
    let _ = interrupted.wait().unwrap();
    assert_eq!(
        observe(repository.root()),
        before,
        "interrupted contention wrote state"
    );
    assert_no_connection(&listener);

    let second_started = Instant::now();
    let second = repository.run_with_env(&["--json", "observe", "query"], &environment);
    assert_public_lock_timeout(
        &second,
        repository.root(),
        second_started.elapsed(),
        timeout,
    );
    assert_eq!(second.status.code(), first.status.code());
    assert_eq!(second.stdout, first.stdout, "contended stdout changed");
    assert_eq!(second.stderr, first.stderr, "contended diagnostic changed");
    assert_eq!(
        observe(repository.root()),
        before,
        "repeat contention wrote state"
    );
    assert_no_connection(&listener);

    holder.unlock().unwrap();
    let released = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(released["event_count"], 1);
    assert_eq!(released["events"][0]["event_id"], "lock-deadline-event");
    assert_eq!(
        released["local_policy"]["lock_timeout_millis"],
        EventStore::supported_lock_timeout_millis()
    );
    repository.teardown();
}

pub(crate) fn assert_public_lock_timeout(
    output: &Output,
    root: &Path,
    elapsed: Duration,
    timeout: Duration,
) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_private_absent(output, root);
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert_eq!(value["exit_class"], "actionable_finding");
    assert_eq!(
        value["cause"],
        "the bounded local event store lock deadline expired before a stable query could begin"
    );
    assert!(
        elapsed >= timeout.saturating_sub(Duration::from_millis(50)),
        "public timeout returned early: {elapsed:?}"
    );
    assert!(
        elapsed <= timeout + Duration::from_secs(2),
        "public timeout exceeded bound: {elapsed:?}"
    );
}

#[test]
pub(crate) fn cargo_supplied_binary_is_a_distinct_regular_executable() {
    let binary = Path::new(env!("CARGO_BIN_EXE_ultragoal"));
    let metadata = fs::symlink_metadata(binary).expect("Cargo binary metadata");
    assert!(metadata.is_file());
    assert!(!metadata.file_type().is_symlink());
    assert_ne!(metadata.permissions().mode() & 0o111, 0);
    assert_ne!(
        fs::canonicalize(binary).unwrap(),
        std::env::current_exe().unwrap()
    );
    let bytes = fs::read(binary).expect("read Cargo binary identity");
    assert!(bytes.len() > 1024);
    let digest = format!("sha256:{:x}", Sha256::digest(bytes));
    assert_eq!(digest.len(), "sha256:".len() + 64);
}
