#[test]
fn separate_process_reservation_race_has_one_winner() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let initial = ledger.head().unwrap();
    let executable = std::env::current_exe().unwrap();
    let hex = b"0123456789abcdef";
    let barrier = fixture.root.with_extension("process-barrier");
    fs::DirBuilder::new().mode(0o700).create(&barrier).unwrap();
    let mut children = Vec::new();
    for value in hex {
        let child = Command::new(&executable)
            .arg("--ignored")
            .arg("--exact")
            .arg("distribution::host_effect::ledger::tests::subprocess_reservation_entrypoint")
            .arg("--nocapture")
            .env("HUL_LEDGER_CHILD_ROOT", &fixture.root)
            .env("HUL_LEDGER_CHILD_HEAD", initial.head_sha256())
            .env(
                "HUL_LEDGER_CHILD_GENERATION",
                initial.generation().to_string(),
            )
            .env("HUL_LEDGER_CHILD_VALUE", char::from(*value).to_string())
            .env("HUL_LEDGER_CHILD_BARRIER", &barrier)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        children.push(child);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let ready = fs::read_dir(&barrier)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.starts_with("ready-"))
            .collect::<BTreeSet<_>>();
        if ready.len() == hex.len() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "only {} of {} child processes reached the pre-flock barrier",
            ready.len(),
            hex.len()
        );
        thread::sleep(std::time::Duration::from_millis(1));
    }
    let release = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(barrier.join("release"))
        .unwrap();
    release.sync_all().unwrap();
    let outputs = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect::<Vec<_>>();
    for output in &outputs {
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let winners = outputs
        .iter()
        .filter(|output| {
            String::from_utf8_lossy(&output.stdout).contains("HUL_LEDGER_CHILD_RESULT=winner")
        })
        .count();
    assert_eq!(winners, 1);
    assert_eq!(ledger.head().unwrap().generation(), 1);
}

#[test]
#[ignore = "subprocess entrypoint invoked only by separate_process_reservation_race_has_one_winner"]
fn subprocess_reservation_entrypoint() {
    let root = PathBuf::from(std::env::var_os("HUL_LEDGER_CHILD_ROOT").unwrap());
    let head_sha256 = std::env::var("HUL_LEDGER_CHILD_HEAD").unwrap();
    let generation = std::env::var("HUL_LEDGER_CHILD_GENERATION")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let value = std::env::var("HUL_LEDGER_CHILD_VALUE")
        .unwrap()
        .chars()
        .next()
        .unwrap();
    let ledger = FileHostEffectLedger::open(&root, "host-ledger".to_owned()).unwrap();
    let head = HostEffectLedgerHead::new(generation, head_sha256).unwrap();
    match ledger.reserve(reservation(&head, value, value, 'f')) {
        Ok(_) => println!("HUL_LEDGER_CHILD_RESULT=winner"),
        Err(error)
            if matches!(
                error.id(),
                HostEffectLedgerErrorId::StaleHead | HostEffectLedgerErrorId::Replay
            ) =>
        {
            println!("HUL_LEDGER_CHILD_RESULT=refused")
        }
        Err(error) => panic!("unexpected child ledger error: {:?}", error.id()),
    }
}

fn overwrite(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

struct LedgerFixture {
    root: PathBuf,
}

impl LedgerFixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        loop {
            let next = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = base.join(format!(
                "hul-host-effect-ledger-{}-{next}",
                std::process::id()
            ));
            match fs::DirBuilder::new().mode(0o700).create(&root) {
                Ok(()) => return Self { root },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create ledger fixture: {error}"),
            }
        }
    }

    fn snapshot(&self) -> Vec<(String, u32, Vec<u8>)> {
        let mut rows = fs::read_dir(&self.root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let metadata = entry.metadata().unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    metadata.permissions().mode() & 0o777,
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.0.cmp(&right.0));
        rows
    }
}

impl Drop for LedgerFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_file(self.root.with_extension("held-lock"));
        let _ = fs::remove_file(self.root.with_extension("held-key"));
        let _ = fs::remove_dir_all(self.root.with_extension("process-barrier"));
    }
}
