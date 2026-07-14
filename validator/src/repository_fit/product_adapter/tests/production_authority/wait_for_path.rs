use super::*;

pub(crate) fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < deadline, "timed out waiting for {path:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(crate) fn result_value(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

pub(crate) fn run_authority_crash(
    fixture: &Fixture,
    role: &str,
    nonce_label: &str,
    exit_code: i32,
) -> PathBuf {
    let result = fixture.container.join(format!("{role}-result.json"));
    let ready = fixture.container.join(format!("{role}-ready"));
    let release = fixture.container.join(format!("{role}-release"));
    let output = authority_scenario_command(fixture, role, nonce_label, &result, &ready, &release)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
    assert!(!result.exists());
    let intent = recovery_intent_path(fixture, nonce_label);
    let bytes = fs::read(&intent).expect("the recovery intent is durable before reservation");
    parse_recovery_intent(&bytes).expect("the persisted recovery intent is canonical");
    intent
}

pub(crate) fn run_authority_scenario(
    fixture: &Fixture,
    role: &str,
    nonce_label: &str,
) -> serde_json::Value {
    run_authority_scenario_at_root(fixture, &fixture.root, role, nonce_label)
}

pub(crate) fn run_authority_scenario_at_root(
    fixture: &Fixture,
    repository_root: &Path,
    role: &str,
    nonce_label: &str,
) -> serde_json::Value {
    let result = fixture.container.join(format!("{role}-result.json"));
    let ready = fixture.container.join(format!("{role}-ready"));
    let release = fixture.container.join(format!("{role}-release"));
    let output = authority_scenario_command_at_root(
        fixture,
        repository_root,
        role,
        nonce_label,
        &result,
        &ready,
        &release,
    )
    .output()
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    result_value(&result)
}

pub(crate) fn observed_root_binding(root: &Path) -> String {
    let mut repository = LocalRepository::open(root).unwrap();
    repository.root_binding().unwrap()
}

pub(crate) fn create_existing_codex_ancestor(fixture: &Fixture) -> PathBuf {
    let path = fixture.root.join(".codex");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

pub(crate) fn replace_directory_preserving_children(path: &Path, quarantine: &Path) {
    let mode = fs::symlink_metadata(path).unwrap().mode() & 0o7777;
    fs::rename(path, quarantine).unwrap();
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    let mut entries = fs::read_dir(quarantine)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        fs::rename(entry.path(), path.join(entry.file_name())).unwrap();
    }
}

pub(crate) fn change_group_if_representable(path: &Path) -> bool {
    let current = fs::symlink_metadata(path).unwrap().gid();
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count <= 0 {
        return false;
    }
    let mut groups = vec![0 as libc::gid_t; count as usize];
    let observed = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
    if observed != count {
        return false;
    }
    let Some(alternate) = groups.into_iter().find(|group| *group != current) else {
        return false;
    };
    let encoded = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let changed = unsafe { libc::chown(encoded.as_ptr(), !0 as libc::uid_t, alternate) } == 0;
    if changed {
        assert_eq!(fs::symlink_metadata(path).unwrap().gid(), alternate);
    }
    changed
}

pub(crate) fn assert_expired_recovery_is_ambiguous(fixture: &Fixture, nonce_label: &str) {
    let recovered = run_authority_scenario(fixture, "recover-expired", nonce_label);
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
pub(crate) fn fresh_exact_apply_commits_and_repeat_is_idempotent() {
    let fixture = Fixture::new("fresh-commit-repeat");
    let context = fixture.context();
    let first = execute(
        &fixture,
        &context,
        fixture.prepared(&context),
        "fresh-commit",
    );
    assert_eq!(first.status(), "applied");
    assert_eq!(first.error_id(), None);
    assert!(first.effect_started());
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }

    let repeated_context = fixture.context();
    let repeated = execute(
        &fixture,
        &repeated_context,
        fixture.prepared(&repeated_context),
        "fresh-idempotent",
    );
    assert_eq!(repeated.status(), "idempotent");
    assert_eq!(repeated.error_id(), None);
    assert!(!repeated.result_id().is_empty());
    assert_eq!(
        fixture.store_names(),
        ["authority-ledger.json", "authority.key", "authority.lock"]
    );
}
