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
    private_test_root(PathBuf::from("/private/tmp").join(format!(
        "hul-evaluation-runtime-086-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst),
    )))
}

fn private_test_root(root: PathBuf) -> PathBuf {
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
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
    assert!((0..participants).all(|index| root.join(format!("ready-{index}")).is_file()));
    fs::write(root.join("release"), b"release").unwrap();
}
