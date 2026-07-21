fn authority(seed: char, version: &str) -> PackageAuthority {
    PackageAuthority {
        version: Version::parse(version).unwrap(),
        package_sha256: digest(seed),
        inventory_sha256: digest('c'),
        candidate_id: digest('d'),
    }
}

fn record_digest(record: &HostLifecycleRecord) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(record).unwrap())
    )
}

fn intent_name(intent: LifecycleIntent) -> &'static str {
    match intent {
        LifecycleIntent::FreshInstall => "fresh_install",
        LifecycleIntent::MonotonicUpdate => "monotonic_update",
        LifecycleIntent::FailedUpdateRecovery => "failed_update_recovery",
        LifecycleIntent::AuthorizedRollback => "authorized_rollback",
        LifecycleIntent::IdempotentReinstall => "idempotent_reinstall",
        LifecycleIntent::UninstallTeardown => "uninstall_teardown",
        LifecycleIntent::StaleCacheRecovery => "stale_cache_recovery",
        LifecycleIntent::RepeatUse => "repeat_use",
    }
}

fn digest(seed: char) -> String {
    format!("sha256:{}", seed.to_string().repeat(64))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct FixtureRoot {
    path: PathBuf,
}

impl FixtureRoot {
    fn new() -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ultragoal-lifecycle-completion-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn remove(self) {
        fs::remove_dir_all(self.path).unwrap();
    }
}
