#[cfg(unix)]
#[test]
fn authorized_effect_enforces_exact_live_executable_identity() {
    let first_fixture = ExecutableFixture::new(b"#!/bin/sh\nexit 0\n");
    let second_fixture = ExecutableFixture::new(b"#!/bin/sh\nexit 0\n");
    let first = PinnedHostExecutable::pin(&first_fixture.path).unwrap();
    let second = PinnedHostExecutable::pin(&second_fixture.path).unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &first)).unwrap();
    authority.verify(&permit, 1_500).unwrap();

    assert_eq!(
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), second, plan)
            .err()
            .unwrap()
            .id(),
        HostEffectLedgerErrorId::InvalidRecord
    );

    let pinned = PinnedHostExecutable::pin(&first_fixture.path).unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &pinned)).unwrap();
    authority.verify(&permit, 1_500).unwrap();
    let authorized =
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), pinned, plan).unwrap();
    authorized.executable().revalidate().unwrap();

    let changed_fixture = ExecutableFixture::new(b"#!/bin/sh\nexit 0\n");
    let changed = PinnedHostExecutable::pin(&changed_fixture.path).unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &changed)).unwrap();
    authority.verify(&permit, 1_500).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .open(&changed_fixture.path)
        .unwrap();
    file.write_all(b"#!/bin/sh\nexit 9\n").unwrap();
    file.sync_all().unwrap();
    assert_eq!(
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), changed, plan)
            .err()
            .unwrap()
            .id(),
        HostEffectLedgerErrorId::Tampered
    );
}

#[cfg(unix)]
struct ExecutableFixture {
    root: std::path::PathBuf,
    path: std::path::PathBuf,
}

#[cfg(unix)]
impl ExecutableFixture {
    fn new(bytes: &[u8]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        let root = loop {
            let attempt = NEXT.fetch_add(1, Ordering::Relaxed);
            let candidate = base.join(format!(
                "hul-host-effect-pin-{}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create executable fixture: {error}"),
            }
        };
        let path = root.join("executable");
        let fixture = Self { root, path };
        fixture.write_executable(&fixture.path, bytes);
        fixture
    }

    fn write_executable(&self, path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[cfg(unix)]
impl Drop for ExecutableFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
