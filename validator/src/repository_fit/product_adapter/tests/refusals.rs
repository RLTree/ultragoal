use super::support::{Fixture, assert_zero_write, snapshot};
use crate::repository_fit::product_adapter::{
    AdapterErrorId, inspect_target, plan_target, prepare_apply_request,
};
use crate::repository_fit::{CanonicalPath, ExpectedContent, FitEffects, FitErrorId, digest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::sync::{Arc, Barrier};

fn sha(byte: u8) -> String {
    format!("sha256:{}", (byte as char).to_string().repeat(64))
}

fn modes() -> BTreeMap<String, u32> {
    BTreeMap::from([
        ("AGENTS.md".to_owned(), 0o644),
        ("nested/AGENTS.md".to_owned(), 0o644),
    ])
}

#[test]
fn noncanonical_unknown_oversized_and_malformed_plan_records_are_zero_write_refusals() {
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
fn stale_plan_context_target_template_and_acceptance_are_distinct_zero_write_refusals() {
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
fn replaced_target_root_is_refused_without_following_the_substitute() {
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
fn local_effects_reject_symlink_hardlink_fifo_socket_and_case_alias_without_mutation() {
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

#[test]
fn leaf_path_swap_and_stale_expected_digest_preserve_foreign_bytes() {
    let fixture = Fixture::new("leaf-swap");
    fixture.write("AGENTS.md", b"observed");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    fs::rename(
        fixture.root.join("AGENTS.md"),
        fixture.root.join("observed-moved"),
    )
    .unwrap();
    fs::write(fixture.root.join("AGENTS.md"), b"foreign").unwrap();
    assert!(
        !effects
            .compare_exchange(
                &CanonicalPath::parse("AGENTS.md").unwrap(),
                &ExpectedContent::ExactDigest(digest(b"observed")),
                Some(b"replacement"),
            )
            .unwrap()
    );
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(fixture.root.join("observed-moved")).unwrap(),
        b"observed"
    );
}

#[test]
fn concurrent_leaf_swap_at_the_atomic_boundary_preserves_all_objects() {
    let fixture = Fixture::new("leaf-linearization-race");
    fixture.write("AGENTS.md", b"observed");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_linearize(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("AGENTS.md"), root.join("observed-moved")).unwrap();
        fs::write(root.join("AGENTS.md"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"observed")),
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"replacement"
    );
    assert_eq!(
        fs::read(fixture.root.join("observed-moved")).unwrap(),
        b"observed"
    );
    let preserved = fs::read_dir(&fixture.root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_file()
                && entry.file_name().to_string_lossy().starts_with(".hul-fit-")
        })
        .map(|entry| fs::read(entry.path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(preserved, vec![b"foreign".to_vec()]);
}

#[test]
fn post_swap_identity_race_preserves_foreign_prior_and_replacement_objects() {
    let fixture = Fixture::new("post-swap-identity-race");
    fixture.write("AGENTS.md", b"prior");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_after_replace_swap(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("AGENTS.md"), root.join("replacement-moved")).unwrap();
        fs::write(root.join("AGENTS.md"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(fixture.root.join("replacement-moved")).unwrap(),
        b"replacement"
    );
    let preserved = fs::read_dir(&fixture.root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".hul-fit-"))
        .map(|entry| fs::read(entry.path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(preserved, vec![b"prior".to_vec()]);
}

#[test]
fn replacement_cleanup_quarantines_a_substituted_temp_without_deleting_it() {
    let fixture = Fixture::new("replacement-cleanup-race");
    fixture.write("AGENTS.md", b"prior");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_quarantine_move(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        let temp = shared_temp_file(&root);
        fs::rename(&temp, root.join("prior-moved")).unwrap();
        fs::write(&temp, b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"replacement"
    );
    assert_eq!(
        fs::read(fixture.root.join("prior-moved")).unwrap(),
        b"prior"
    );
    assert_eq!(
        quarantined_objects(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}

#[test]
fn removal_cleanup_quarantines_a_substituted_temp_without_deleting_it() {
    let fixture = Fixture::new("removal-cleanup-race");
    fixture.write("AGENTS.md", b"prior");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_quarantine_move(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        let temp = shared_temp_file(&root);
        fs::rename(&temp, root.join("prior-moved")).unwrap();
        fs::write(&temp, b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            None,
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert!(!fixture.root.join("AGENTS.md").exists());
    assert_eq!(
        fs::read(fixture.root.join("prior-moved")).unwrap(),
        b"prior"
    );
    assert_eq!(
        quarantined_objects(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}

#[test]
fn staged_parent_publish_refuses_a_substituted_directory_without_writing_into_it() {
    let fixture = Fixture::new("parent-publish-race");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_after_directory_publish(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("nested"), root.join("managed-moved")).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/marker"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
            &ExpectedContent::Absent,
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("nested/marker")).unwrap(),
        b"foreign"
    );
    assert!(fixture.root.join("managed-moved").is_dir());
    assert!(!fixture.root.join("nested/AGENTS.md").exists());
}

#[test]
fn created_parent_cleanup_quarantines_a_substituted_directory_without_deleting_it() {
    let fixture = Fixture::new("parent-cleanup-race");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    assert!(
        effects
            .compare_exchange(
                &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
                &ExpectedContent::Absent,
                Some(b"managed"),
            )
            .unwrap()
    );
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_directory_quarantine_move(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("nested"), root.join("managed-moved")).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/marker"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"managed")),
            None,
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert!(fixture.root.join("managed-moved").is_dir());
    assert_eq!(
        quarantined_directory_markers(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}

fn shared_temp_file(root: &std::path::Path) -> std::path::PathBuf {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry.file_type().unwrap().is_file()
                && entry.file_name().to_string_lossy().starts_with(".hul-fit-")
        })
        .unwrap()
        .path()
}

fn quarantined_objects(root: &std::path::Path) -> Vec<Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_dir()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".hul-fit-transaction-")
        })
        .map(|entry| fs::read(entry.path().join("object")).unwrap())
        .collect()
}

fn quarantined_directory_markers(root: &std::path::Path) -> Vec<Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_dir()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".hul-fit-transaction-")
        })
        .filter_map(|entry| fs::read(entry.path().join("object/marker")).ok())
        .collect()
}

#[test]
fn cross_device_guard_fails_closed_before_any_namespace_effect() {
    assert!(crate::repository_fit::local::LocalEffects::test_device_guard(42, 42).is_ok());
    assert_eq!(
        crate::repository_fit::local::LocalEffects::test_device_guard(42, 43)
            .unwrap_err()
            .id(),
        FitErrorId::UnsafeObject
    );
}

#[test]
fn production_local_effects_require_a_root_owned_exclusive_mutation_lease() {
    let fixture = Fixture::new("missing-exclusive-mutation-lease");
    fixture.write("AGENTS.md", b"prior");
    let before = snapshot(&fixture.root);
    let mut effects =
        crate::repository_fit::local::LocalEffects::open(&fixture.root, modes()).unwrap();
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            Some(b"replacement"),
        )
        .unwrap_err();
    assert_eq!(failure.id(), FitErrorId::Unauthorized);
    assert_eq!(snapshot(&fixture.root), before);
}
