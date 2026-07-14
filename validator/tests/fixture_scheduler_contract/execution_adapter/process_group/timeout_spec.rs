fn timeout_spec(id: &str, wall_time_millis: u64) -> FixtureSpec {
    FixtureSpec::new_with_confinement(
        id,
        FixtureKind::Positive,
        "confined-capture-timeout",
        BTreeSet::from([
            ResourceKind::File,
            ResourceKind::Env,
            ResourceKind::Process,
            ResourceKind::Port,
        ]),
        ExpectedOutcome::pass(2),
        false,
        ConfinementPolicy {
            cpu_seconds: 5,
            address_space_bytes: 64 * 1024 * 1024,
            wall_time_millis,
            maximum_file_bytes: 1024 * 1024,
            require_process_group: true,
            network: NetworkIsolation::DenyAll,
        },
    )
    .unwrap()
}

pub(super) fn compile_probe(root: &Path) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let output = root.join("confinement-probe");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixture_scheduler_contract/confinement/probe.rs");
    assert!(
        Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    output
}

#[derive(Debug)]
pub(super) struct ProcessRecord {
    pub(super) group: i32,
    pub(super) descendants: Vec<i32>,
    pub(super) denied: usize,
}

pub(super) fn read_process_record(path: &Path) -> ProcessRecord {
    let contents = std::fs::read_to_string(path).unwrap();
    let mut lines = contents.lines();
    let group = lines
        .next()
        .unwrap()
        .strip_prefix("group=")
        .unwrap()
        .parse()
        .unwrap();
    let descendants = lines
        .next()
        .unwrap()
        .strip_prefix("descendants=")
        .unwrap()
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().unwrap())
        .collect();
    let denied = lines
        .next()
        .unwrap()
        .strip_prefix("denied=")
        .unwrap()
        .parse()
        .unwrap();
    ProcessRecord {
        group,
        descendants,
        denied,
    }
}

fn wait_for_readiness(path: &Path) {
    const READY: &[u8] = b"ready\n";
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                assert!(
                    metadata.file_type().is_file(),
                    "readiness handshake is not a regular file"
                );
                let bytes = std::fs::read(path).expect("read readiness handshake");
                assert!(
                    READY.starts_with(&bytes),
                    "unexpected readiness handshake bytes: {bytes:?}"
                );
                if bytes == READY {
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("inspect readiness handshake: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "fixture never reached the readiness handshake"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn signal_target_absent(target: i32) -> bool {
    if unsafe { libc::kill(target, 0) } == 0 {
        return false;
    }
    let error = std::io::Error::last_os_error();
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH), "{error}");
    true
}

pub(super) fn group_and_descendants_absent(record: &ProcessRecord) -> bool {
    signal_target_absent(-record.group)
        && record
            .descendants
            .iter()
            .all(|pid| signal_target_absent(*pid))
}

pub(super) fn wait_for_natural_test_cleanup(record: &ProcessRecord) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && !group_and_descendants_absent(record) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(group_and_descendants_absent(record));
}

#[derive(Clone, Copy)]
enum TerminationTrigger {
    WallTime,
    Interrupt,
    OutputLimit,
}
