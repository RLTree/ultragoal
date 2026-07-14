static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn review_id(binding_sha256: &str, attestation_sha256: &str) -> String {
    digest(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes())
}

fn root(label: &str) -> PathBuf {
    let root = PathBuf::from("/private/tmp").join(format!(
        "hul-evaluation-runtime-086-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
}

fn fixture_spec(id: &str) -> FixtureSpec {
    FixtureSpec::new(
        id,
        FixtureKind::Positive,
        "evaluation-runtime-contract",
        BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
        ExpectedOutcome::pass(0),
        false,
    )
    .unwrap()
}

fn write_shell(path: &Path, output: &str) {
    fs::write(path, format!("#!/bin/sh\nprintf '%s' '{output}'\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o500)).unwrap();
}

fn wait_until(label: &str, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn process_barrier() -> Result<(), ()> {
    let Some(root) = std::env::var_os("HUL_EVAL_RACE_BARRIER") else {
        return Ok(());
    };
    let participant = std::env::var("HUL_EVAL_RACE_PARTICIPANT").map_err(|_| ())?;
    let root = PathBuf::from(root);
    fs::write(root.join(format!("ready-{participant}")), b"ready").map_err(|_| ())?;
    let release = root.join("release");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !release.is_file() {
        if Instant::now() >= deadline {
            return Err(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

fn release_process_barrier(root: &Path, participants: usize) {
    wait_until("process race participants", || {
        (0..participants).all(|index| root.join(format!("ready-{index}")).is_file())
    });
    fs::write(root.join("release"), b"release").unwrap();
}

#[test]
fn fixture_permit_rejects_issue_time_path_substitution() {
    let root = root("fixture-issue-swap");
    let original = root.join("fixture.sh");
    let accepted = root.join("accepted.sh");
    let replacement = root.join("replacement.sh");
    write_shell(&original, "accepted");
    write_shell(&replacement, "substituted");
    let fixture = fixture_spec("fixture-issue-swap");

    FixtureCaptureAdapter::set_test_issue_pause(original.clone(), 500);
    let issue_path = original.clone();
    let issuance = std::thread::spawn(move || {
        FixtureCaptureAdapter::issue(
            &fixture,
            issue_path,
            Vec::<OsString>::new(),
            4096,
            b"accepted".to_vec(),
        )
    });
    wait_until("fixture permit issue pause", || {
        FixtureCaptureAdapter::test_issue_is_paused()
    });
    fs::rename(&original, &accepted).unwrap();
    fs::rename(&replacement, &original).unwrap();
    let error = match issuance.join().unwrap() {
        Ok(_) => panic!("path substitution unexpectedly retained a fixture permit"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        fixture_scheduler::FixtureScheduleError::Integrity(_)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn captured_shell_bytes_survive_pre_launch_path_substitution() {
    let root = root("fixture-launch-swap");
    let source = root.join("fixture.sh");
    let accepted = root.join("accepted.sh");
    let replacement = root.join("replacement.sh");
    write_shell(&source, "accepted");
    write_shell(&replacement, "substituted");
    let fixture = fixture_spec("fixture-launch-swap");
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        source.clone(),
        Vec::<OsString>::new(),
        4096,
        b"accepted".to_vec(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease_id = scheduler.schedule([fixture]).unwrap().pop().unwrap();

    FixtureCaptureAdapter::set_test_pre_launch_pause(source.clone(), 500);
    let execution = std::thread::spawn(move || {
        scheduler
            .execute_recorded(&lease_id, &adapter)
            .map(|(disposition, record)| (disposition, record.artifact_bytes().to_vec()))
    });
    wait_until("fixture pre-launch pause", || {
        FixtureCaptureAdapter::test_pre_launch_is_paused()
    });
    fs::rename(&source, &accepted).unwrap();
    fs::rename(&replacement, &source).unwrap();

    let (disposition, artifact) = execution.join().unwrap().unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::Accepted);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(artifact, b"accepted");
    assert_ne!(artifact, b"substituted");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_shell_route_enforces_the_process_group_wall_timeout() {
    let root = root("fixture-shell-timeout");
    let source = root.join("fixture.sh");
    fs::write(&source, b"#!/bin/sh\nwhile :; do :; done\n").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o500)).unwrap();
    let fixture = FixtureSpec::new_with_confinement(
        "fixture-shell-timeout",
        FixtureKind::Negative,
        "evaluation-runtime-contract",
        BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Process]),
        ExpectedOutcome::causal_failure("fixture-wall-time-exceeded", 0),
        false,
        ConfinementPolicy {
            cpu_seconds: 5,
            address_space_bytes: 64 * 1024 * 1024,
            wall_time_millis: 100,
            maximum_file_bytes: 1024 * 1024,
            require_process_group: true,
            network: NetworkIsolation::DenyAll,
        },
    )
    .unwrap();
    let adapter =
        FixtureCaptureAdapter::issue(&fixture, source, Vec::<OsString>::new(), 4096, Vec::new())
            .unwrap();
    let mut scheduler = FixtureScheduler::new(root.join("leases"));
    let lease_id = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    let started = Instant::now();
    let (disposition, record) = scheduler.execute_recorded(&lease_id, &adapter).unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::CausalFailure);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(record.exit_code, None);
    assert_eq!(record.outcome.causal_code, "fixture-wall-time-exceeded");
    assert_eq!(record.outcome.claim_ceiling, 0);
    assert!(started.elapsed() < Duration::from_secs(2));
    fs::remove_dir_all(root).unwrap();
}
