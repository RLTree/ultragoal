#[cfg(unix)]
#[test]
fn link_hardlink_fifo_and_socket_install_objects_refuse_without_outside_write() {
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("special-{kind}"));
        let bundle = fixture.bundle("0.0.12");
        let installed_dir = fixture.root.join("installed");
        std::fs::create_dir_all(&installed_dir).unwrap();
        let target = installed_dir.join("harness-ultragoal.hugpkg");
        let outside = fixture.root.join("outside-canary");
        std::fs::write(&outside, b"outside-canary\n").unwrap();
        let _listener = match kind {
            "symlink" => {
                symlink(&outside, &target).unwrap();
                None
            }
            "hardlink" => {
                std::fs::hard_link(&outside, &target).unwrap();
                None
            }
            "fifo" => {
                assert!(
                    std::process::Command::new("mkfifo")
                        .arg(&target)
                        .status()
                        .unwrap()
                        .success()
                );
                None
            }
            "socket" => Some(UnixListener::bind(&target).unwrap()),
            _ => unreachable!(),
        };
        let logical = installed(&bundle.authority, 3);
        let repeat = lifecycle(
            &logical,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                logical.installed.as_ref(),
                false,
                false,
            ),
        );
        let before = std::fs::read(&outside).unwrap();
        let result = std::panic::catch_unwind(|| fixture.session(&bundle, repeat));
        assert!(result.is_err(), "{kind} unexpectedly bound");
        assert_eq!(std::fs::read(&outside).unwrap(), before);
    }
}

#[test]
fn oversized_and_attacker_observations_fail_without_echo_or_mutation() {
    let fixture = Fixture::new("attacker-observation");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 4);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, _) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let canary = "SECRET_ATTACKER_CANARY";
    let mut reader = Reader {
        provenance_sha256: session.binding().binding_sha256().to_owned(),
        marketplace: Some(format!("{canary}{}", "x".repeat(4 * 1024 * 1024)).into_bytes()),
        ..Reader::default()
    };
    let before = fixture.tree();
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::ObservationConflict);
    assert!(!error.to_string().contains(canary));
    assert_eq!(fixture.tree(), before);
}

#[test]
fn external_effect_requests_are_candidate_bound_and_never_authorizations() {
    let fixture = Fixture::new("effect-request-binding");
    let first = fixture.bundle("0.0.11");
    let second = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let first_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(first.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut first_session, _) = fixture.session(&first, first_plan);
    first_session.apply_confined(&empty).unwrap();
    let first_request = first_session.take_external_effect_request().unwrap();

    let other_fixture = Fixture::new("effect-request-binding-other");
    let other_second = other_fixture.bundle("0.0.12");
    let second_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(other_second.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut second_session, _) = other_fixture.session(&other_second, second_plan);
    second_session.apply_confined(&empty).unwrap();
    let second_request = second_session.take_external_effect_request().unwrap();
    assert_ne!(
        first_request.request_sha256(),
        second_request.request_sha256()
    );
    assert_ne!(
        first.snapshot.package_sha256(),
        second.snapshot.package_sha256()
    );
    assert!(!first_request.plan_sha256().is_empty());
    let serialized = serde_json::to_value(&first_request).unwrap();
    assert_eq!(
        serialized["session_issuance_sha256"],
        first_request.session_issuance_sha256()
    );
    assert!(serialized.get("commands").is_none());
    assert!(serialized.get("plan").is_none());
    let first_prepared = first_session
        .consume_external_effect_request(first_request)
        .unwrap();
    let first_host_plan = first_prepared.into_plan().unwrap();
    assert_eq!(first_host_plan.commands()[0].program(), "codex");
    let second_prepared = second_session
        .consume_external_effect_request(second_request)
        .unwrap();
    assert_eq!(second_prepared.into_plan().unwrap().commands().len(), 1);
}

#[test]
fn observation_capture_mutation_is_zero_write_and_does_not_consume_session() {
    let fixture = Fixture::new("capture-mutation");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 5);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();
    let mut raced = Reader::complete(&bundle, &host, session.binding());
    raced.mutate_marketplace_after_first = true;
    assert_eq!(
        session.capture_and_verify(&mut raced).unwrap_err().id(),
        HostLifecycleErrorId::ObservationChanged
    );
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
}
