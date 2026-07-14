#[test]
fn concurrent_duplicate_fixture_lease_is_rejected_and_recovery_releases_it() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler.schedule([spec("same")]).unwrap().pop().unwrap();
    assert!(matches!(
        scheduler.schedule([spec("same")]),
        Err(FixtureScheduleError::Collision(_))
    ));
    assert_exact_namespaces(scheduler.run(&id).unwrap(), &root, &id, "same");

    #[cfg(target_os = "freebsd")]
    {
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CausalFailure
        );
        assert_eq!(scheduler.active_count(), 0);
        assert!(!root.join(&id).exists());
        assert!(matches!(
            scheduler.recover(&id),
            Err(FixtureScheduleError::UnknownLease(observed)) if observed == id
        ));
    }
    #[cfg(not(target_os = "freebsd"))]
    {
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CleanupFailure
        );
        assert_eq!(scheduler.active_count(), 1);
        assert_recovery_required(&scheduler, &root, &id, "same");
        assert_eq!(
            scheduler.recover(&id).unwrap(),
            RunDisposition::CleanupFailure
        );
        assert_eq!(scheduler.active_count(), 1);
        assert_recovery_required(&scheduler, &root, &id, "same");
        assert!(matches!(
            scheduler.schedule([spec("same")]),
            Err(FixtureScheduleError::Collision(_))
        ));
    }
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn newly_scheduled_lease_is_returned_even_when_another_lease_sorts_later() {
    let root = root();
    let mut scheduler = FixtureScheduler::new(&root);
    let later = scheduler.schedule([spec("zeta")]).unwrap().pop().unwrap();
    let earlier = scheduler.schedule([spec("alpha")]).unwrap().pop().unwrap();
    assert!(earlier.starts_with("alpha-"));
    assert_eq!(
        scheduler.run(&later).unwrap().lease.root(),
        root.join(&later)
    );
    assert_eq!(
        scheduler.run(&earlier).unwrap().lease.root(),
        root.join(&earlier)
    );
    assert_exact_namespaces(scheduler.run(&later).unwrap(), &root, &later, "zeta");
    assert_exact_namespaces(scheduler.run(&earlier).unwrap(), &root, &earlier, "alpha");

    #[cfg(target_os = "freebsd")]
    let expected_after_earlier = 1;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_earlier = 2;
    finish_and_recover_on_platform(
        &mut scheduler,
        &root,
        &earlier,
        "alpha",
        expected_after_earlier,
    );
    #[cfg(target_os = "freebsd")]
    let expected_after_later = 0;
    #[cfg(not(target_os = "freebsd"))]
    let expected_after_later = 2;
    finish_and_recover_on_platform(&mut scheduler, &root, &later, "zeta", expected_after_later);
    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}
