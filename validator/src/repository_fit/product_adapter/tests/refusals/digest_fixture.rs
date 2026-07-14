use super::*;

pub(crate) fn sha(byte: u8) -> String {
    format!("sha256:{}", (byte as char).to_string().repeat(64))
}

pub(crate) fn modes() -> BTreeMap<String, u32> {
    BTreeMap::from([
        ("AGENTS.md".to_owned(), 0o644),
        ("nested/AGENTS.md".to_owned(), 0o644),
    ])
}

#[test]
pub(crate) fn noncanonical_unknown_oversized_and_malformed_plan_records_are_zero_write_refusals() {
    let fixture = Fixture::new("plan-record-refusal");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();
    let canonical = record.to_machine_bytes().unwrap();
    let accepted = record.plan_sha256().to_owned();

    let mut noncanonical = canonical.clone();
    noncanonical.push(b'\n');
    let mut value: Value = serde_json::from_slice(&canonical).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_owned(), Value::Bool(true));
    let unknown = serde_json::to_vec(&value).unwrap();
    for bytes in [
        noncanonical,
        unknown,
        b"not-json".to_vec(),
        vec![b'x'; 16 * 1024 * 1024 + 1],
    ] {
        let failure = assert_zero_write(&fixture, || {
            prepare_apply_request(&context, &bytes, &accepted)
                .err()
                .unwrap()
        });
        assert_eq!(failure.id(), AdapterErrorId::InvalidPlanRecord);
    }
}

#[test]
pub(crate) fn stale_plan_context_target_template_and_acceptance_are_distinct_zero_write_refusals() {
    let fixture = Fixture::new("stale-controls");
    let context = fixture.context();
    let record = plan_target(&context).unwrap();

    let mut stale_plan = record.clone();
    stale_plan.plan.plan_sha256 = sha(b'a');
    let stale_plan_bytes = stale_plan.to_machine_bytes().unwrap();
    let failure = assert_zero_write(&fixture, || {
        prepare_apply_request(&context, &stale_plan_bytes, &sha(b'a'))
            .err()
            .unwrap()
    });
    assert_eq!(failure.id(), AdapterErrorId::StalePlan);

    let mut stale_target = record.clone();
    stale_target.target.worktree_root_id = sha(b'b');
    let failure = assert_zero_write(&fixture, || {
        prepare_apply_request(
            &context,
            &stale_target.to_machine_bytes().unwrap(),
            record.plan_sha256(),
        )
        .err()
        .unwrap()
    });
    assert_eq!(failure.id(), AdapterErrorId::StalePlan);

    let mut stale_template = record.clone();
    stale_template.authority.manifest_sha256 = sha(b'c');
    let failure = assert_zero_write(&fixture, || {
        prepare_apply_request(
            &context,
            &stale_template.to_machine_bytes().unwrap(),
            record.plan_sha256(),
        )
        .err()
        .unwrap()
    });
    assert_eq!(failure.id(), AdapterErrorId::StalePlan);

    let failure = assert_zero_write(&fixture, || {
        prepare_apply_request(&context, &record.to_machine_bytes().unwrap(), &sha(b'd'))
            .err()
            .unwrap()
    });
    assert_eq!(failure.id(), AdapterErrorId::AcceptanceMismatch);

    fixture.write("candidate-changed", b"new candidate");
    let before = snapshot(&fixture.root);
    let failure = inspect_target(&context).err().unwrap();
    assert_eq!(failure.id(), AdapterErrorId::ContextStale);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn replaced_target_root_is_refused_without_following_the_substitute() {
    let fixture = Fixture::new("root-swap");
    let context = fixture.context();
    let moved = fixture.container.join("moved-repo");
    fs::rename(&fixture.root, &moved).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    fs::write(fixture.root.join("AGENTS.md"), b"substitute").unwrap();
    let substitute = snapshot(&fixture.root);
    let failure = inspect_target(&context).err().unwrap();
    assert_eq!(failure.id(), AdapterErrorId::ContextStale);
    assert_eq!(snapshot(&fixture.root), substitute);
}

#[test]
pub(crate) fn local_effects_reject_symlink_hardlink_fifo_socket_and_case_alias_without_mutation() {
    type Builder = fn(&Fixture) -> bool;
    fn symlink_fixture(fixture: &Fixture) -> bool {
        fixture.write("real", b"real");
        symlink("real", fixture.root.join("AGENTS.md")).unwrap();
        true
    }
    fn hardlink_fixture(fixture: &Fixture) -> bool {
        fixture.write("real", b"real");
        fs::hard_link(fixture.root.join("real"), fixture.root.join("AGENTS.md")).unwrap();
        true
    }
    fn fifo_fixture(fixture: &Fixture) -> bool {
        let path = fixture.root.join("AGENTS.md");
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        true
    }
    fn socket_fixture(fixture: &Fixture) -> bool {
        let _listener =
            std::os::unix::net::UnixListener::bind(fixture.root.join("AGENTS.md")).unwrap();
        true
    }
    fn alias_fixture(fixture: &Fixture) -> bool {
        fixture.write("AGENTS.md", b"exact");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(fixture.root.join("Agents.md"))
        {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => panic!("alias fixture failed: {error}"),
        }
    }

    let builders: [(&str, Builder); 5] = [
        ("effect-symlink", symlink_fixture),
        ("effect-hardlink", hardlink_fixture),
        ("effect-fifo", fifo_fixture),
        ("effect-socket", socket_fixture),
        ("effect-alias", alias_fixture),
    ];
    for (label, build) in builders {
        let fixture = Fixture::new(label);
        if !build(&fixture) {
            continue;
        }
        let before = snapshot(&fixture.root);
        let mut effects =
            crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes())
                .unwrap();
        let failure = effects
            .compare_exchange(
                &CanonicalPath::parse("AGENTS.md").unwrap(),
                &ExpectedContent::ExactDigest(digest(b"exact")),
                Some(b"replacement"),
            )
            .err()
            .unwrap();
        assert!(
            matches!(
                failure.id(),
                FitErrorId::UnsafeObject | FitErrorId::StaleBinding
            ),
            "{label}: {failure:?}"
        );
        assert_eq!(snapshot(&fixture.root), before, "{label}");
    }
}
