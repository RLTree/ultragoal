use super::*;

#[derive(Deserialize)]
struct Matrix {
    schema: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
}

#[test]
fn declared_adversarial_matrix_is_unique_and_exercised_by_this_contract() {
    let matrix: Matrix = serde_json::from_str(include_str!(
        "../../../fixtures/supported-host-plugin-transaction/adversarial-cases.json"
    ))
    .unwrap();
    assert_eq!(
        matrix.schema,
        "harness-ultragoal.supported-host-adversarial-cases.v1"
    );
    let unique = matrix
        .cases
        .iter()
        .map(|row| row.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), matrix.cases.len());
    assert!(matrix.cases.len() >= 18);
}

#[test]
fn root_alias_and_marketplace_traversal_never_gain_host_authority() {
    let fixture = Fixture::new("root-alias");
    let alias = fixture.root.with_file_name(format!(
        "hul-distribution-supported-host-alias-{}",
        std::process::id()
    ));
    symlink(&fixture.root, &alias).unwrap();
    assert!(ConfinedRoot::open(&alias).is_err());
    fs::remove_file(&alias).unwrap();

    let package = fixture.bundle("0.0.12");
    assert_eq!(
        fixture
            .adapter
            .plan_install(&package.snapshot, "../local-harness-plugins")
            .unwrap_err()
            .id(),
        DarwinHostErrorId::InvalidMarketplace
    );
}

#[test]
fn symlink_hardlink_fifo_socket_special_and_unsafe_mode_are_rejected() {
    let symlink_fixture = installed_fixture("unsafe-symlink");
    symlink_fixture.remove_surface(DarwinHostSurface::Installed);
    fs::create_dir_all(
        symlink_fixture
            .root
            .join(DarwinHostSurface::Installed.relative_path()),
    )
    .unwrap();
    symlink(
        "/etc/hosts",
        symlink_fixture.record_path(DarwinHostSurface::Installed),
    )
    .unwrap();
    assert_unsafe(&symlink_fixture, DarwinHostSurface::Installed);

    let hardlink_fixture = installed_fixture("unsafe-hardlink");
    fs::hard_link(
        hardlink_fixture.record_path(DarwinHostSurface::Installed),
        hardlink_fixture
            .record_path(DarwinHostSurface::Installed)
            .with_file_name("record-alias"),
    )
    .unwrap();
    assert_unsafe(&hardlink_fixture, DarwinHostSurface::Installed);

    let fifo_fixture = installed_fixture("unsafe-fifo");
    fifo_fixture.remove_surface(DarwinHostSurface::Installed);
    fs::create_dir_all(
        fifo_fixture
            .root
            .join(DarwinHostSurface::Installed.relative_path()),
    )
    .unwrap();
    let fifo = fifo_fixture.record_path(DarwinHostSurface::Installed);
    let fifo_text = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_text.as_ptr(), 0o600) }, 0);
    assert_unsafe(&fifo_fixture, DarwinHostSurface::Installed);

    let socket_fixture = installed_fixture("unsafe-socket");
    socket_fixture.remove_surface(DarwinHostSurface::Installed);
    fs::create_dir_all(
        socket_fixture
            .root
            .join(DarwinHostSurface::Installed.relative_path()),
    )
    .unwrap();
    let _socket =
        UnixListener::bind(socket_fixture.record_path(DarwinHostSurface::Installed)).unwrap();
    assert_unsafe(&socket_fixture, DarwinHostSurface::Installed);

    let mode_fixture = installed_fixture("unsafe-mode");
    mode_fixture.chmod_record(DarwinHostSurface::Installed, 0o666);
    assert_unsafe(&mode_fixture, DarwinHostSurface::Installed);

    let special_fixture = Fixture::new("unsafe-special-model");
    let rows = vec![TreeObject::adversarial(
        "record".into(),
        TreeObjectKind::Special,
        0o644,
        1,
        Vec::new(),
    )];
    assert_eq!(
        special_fixture
            .adapter
            .validate_rows_for_test(DarwinHostSurface::Installed, &rows)
            .unwrap_err()
            .id(),
        DarwinHostErrorId::UnsafeObject
    );
}

#[test]
fn oversized_surface_and_case_fold_collision_are_rejected_without_mutation() {
    let oversized = installed_fixture("oversized");
    let record = oversized.record_path(DarwinHostSurface::Installed);
    let file = fs::OpenOptions::new().write(true).open(&record).unwrap();
    file.set_len((65 * 1024 * 1024 + 1) as u64).unwrap();
    let plan = installed_plan(&oversized);
    let error = oversized.adapter.query(&plan).unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::ObjectTooLarge);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Installed));

    let collision = Fixture::new("casefold-collision");
    let rows = vec![
        TreeObject::regular("record".into(), 0o644, b"first".to_vec()),
        TreeObject::regular("Record".into(), 0o644, b"second".to_vec()),
    ];
    let error = collision
        .adapter
        .validate_rows_for_test(DarwinHostSurface::Discovery, &rows)
        .unwrap_err();
    assert_eq!(error.id(), DarwinHostErrorId::UnsafeObject);
    assert_eq!(error.surface(), Some(DarwinHostSurface::Discovery));
}

#[test]
fn unknown_duplicate_and_cross_surface_records_are_never_accepted() {
    let unknown = installed_fixture("unknown-field");
    let surface = DarwinHostSurface::Marketplace;
    let mut bytes = fs::read(unknown.record_path(surface)).unwrap();
    bytes.pop();
    bytes.extend_from_slice(b",\"unknown\":true}");
    unknown.overwrite_record(surface, &bytes);
    let plan = installed_plan(&unknown);
    assert_eq!(
        unknown
            .adapter
            .query(&plan)
            .unwrap()
            .surface(surface)
            .status(),
        DarwinSurfaceStatus::Dirty
    );

    let duplicate = installed_fixture("duplicate-field");
    let surface = DarwinHostSurface::Marketplace;
    let bytes = fs::read(duplicate.record_path(surface)).unwrap();
    let mut replacement = b"{\"schema\":\"duplicate\",".to_vec();
    replacement.extend_from_slice(&bytes[1..]);
    duplicate.overwrite_record(surface, &replacement);
    let plan = installed_plan(&duplicate);
    assert_eq!(
        duplicate
            .adapter
            .query(&plan)
            .unwrap()
            .surface(surface)
            .status(),
        DarwinSurfaceStatus::Dirty
    );

    let crossed = installed_fixture("cross-surface");
    let registry = fs::read(crossed.record_path(DarwinHostSurface::AppRegistry)).unwrap();
    crossed.overwrite_record(DarwinHostSurface::Discovery, &registry);
    let plan = installed_plan(&crossed);
    let snapshot = crossed.adapter.query(&plan).unwrap();
    assert_eq!(snapshot.diagnosis(), DarwinHostDiagnosis::Conflict);
    assert_eq!(
        snapshot.discovery().status(),
        DarwinSurfaceStatus::CrossSurface
    );
}
