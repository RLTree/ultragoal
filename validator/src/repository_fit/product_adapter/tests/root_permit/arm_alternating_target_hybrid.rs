use super::*;

pub(crate) fn arm_alternating_target_hybrid(
    first_path: &'static str,
    second_path: &'static str,
    first: PathBuf,
    second: PathBuf,
    first_desired: Vec<u8>,
    first_other: Vec<u8>,
    second_desired: Vec<u8>,
    second_other: Vec<u8>,
    remaining_collects: usize,
) {
    if remaining_collects == 0 {
        return;
    }
    target_capture_hook_for_test(
        TargetCapturePhase::AfterLeafRevalidated,
        first_path,
        move || {
            fs::write(&first, &first_other).unwrap();
            fs::write(&second, &second_other).unwrap();
            target_capture_hook_for_test(
                TargetCapturePhase::AfterLeafRevalidated,
                second_path,
                move || {
                    fs::write(&second, &second_desired).unwrap();
                    fs::write(&first, &first_desired).unwrap();
                    arm_alternating_target_hybrid(
                        first_path,
                        second_path,
                        first,
                        second,
                        first_desired,
                        first_other,
                        second_desired,
                        second_other,
                        remaining_collects - 1,
                    );
                },
            );
        },
    );
}

pub(crate) fn same_length_other(bytes: &[u8]) -> Vec<u8> {
    let mut other = bytes.to_vec();
    let byte = other
        .first_mut()
        .expect("canonical target fixtures are nonempty");
    *byte ^= 1;
    other
}

pub(crate) fn assert_target_complete_collect_race(
    label: &str,
    mutate: impl FnOnce(&Path, &Path, &Path) + 'static,
) {
    let fixture = Fixture::new(label);
    fixture.install_all();
    let context = fixture.context();
    let request = fixture.request(&context);
    let target = fixture.root.join("AGENTS.md");
    let root = fixture.root.clone();
    let container = fixture.container.clone();
    target_capture_hook_for_test(TargetCapturePhase::BeforeFinalChainRecheck, "", move || {
        mutate(&target, &root, &container)
    });
    let result = new_authority().issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce(label),
    );
    assert_target_capture_hook_consumed_for_test();
    let failure = match result {
        Err(failure) => failure,
        Ok(_) => panic!("a {label} target race unexpectedly issued authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::TargetUnavailable, "{label}");
}

pub(crate) fn alternate_supplementary_gid(current: u32) -> u32 {
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    assert!(count > 0, "the target-race test requires a local group");
    let mut groups = vec![0 as libc::gid_t; count as usize];
    assert_eq!(
        unsafe { libc::getgroups(count, groups.as_mut_ptr()) },
        count
    );
    groups
        .into_iter()
        .map(|group| group as u32)
        .find(|group| *group != current)
        .expect("the target-race test requires an alternate supplementary group")
}

pub(crate) fn arm_protected_capture_barrier(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: &[u8],
    mutate: impl FnOnce() + Send + 'static,
) -> JoinHandle<()> {
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let attacker_reached = Arc::clone(&reached);
    let attacker_resume = Arc::clone(&resume);
    let attacker = std::thread::spawn(move || {
        attacker_reached.wait();
        mutate();
        attacker_resume.wait();
    });
    protected_capture_hook_for_test(boundary, phase, path, move || {
        reached.wait();
        resume.wait();
    });
    attacker
}

pub(crate) fn protected_directory_swap(
    fixture: &Fixture,
    label: &str,
) -> (PathBuf, PathBuf, PathBuf) {
    let active = fixture.root.join("private");
    let replacement = fixture.container.join(format!("{label}-protected-b"));
    let displaced = fixture.container.join(format!("{label}-protected-a"));
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("preserved.txt"), b"protected A bytes\n").unwrap();
    fs::create_dir_all(&replacement).unwrap();
    fs::write(replacement.join("preserved.txt"), b"protected A bytes\n").unwrap();
    (active, replacement, displaced)
}

pub(crate) fn swap_protected_directories(
    active: PathBuf,
    replacement: PathBuf,
    displaced: PathBuf,
) {
    fs::rename(&active, displaced).unwrap();
    fs::rename(replacement, active).unwrap();
}

pub(crate) fn change_version(path: &Path) -> (i64, i64) {
    let metadata = fs::metadata(path).unwrap();
    (metadata.ctime(), metadata.ctime_nsec())
}

pub(crate) fn assert_late_regular_write_remains(
    path: &Path,
    expected_inode: u64,
    expected_bytes: &[u8],
    original_version: (i64, i64),
) {
    let metadata = fs::metadata(path).unwrap();
    assert_eq!(metadata.ino(), expected_inode);
    assert_eq!(fs::read(path).unwrap(), expected_bytes);
    assert_ne!(change_version(path), original_version);
}

pub(crate) fn assert_pre_effect_ancestor_drift(
    label: &str,
    ancestor: &str,
    mutate: impl FnOnce(&Fixture, &Path),
) {
    let fixture = Fixture::new(label);
    let path = fixture.root.join(ancestor);
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    mutate(&fixture, &path);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started(), "{label}");
}
