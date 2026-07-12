use crate::support::{Fixture, lifecycle, request};
use std::fs;
use ultragoal::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};

#[test]
fn symlink_substitution_and_attacker_value_fail_closed_without_outside_write() {
    let fixture = Fixture::new("symlink-secret-canary");
    let bundle = fixture.bundle("0.0.11");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&bundle, &plan);
    let canary = fixture.root.with_file_name("SECRET_PLUGIN_ADAPTER_CANARY");
    fs::write(&canary, b"outside\n").unwrap();
    fs::create_dir(fixture.root.join("installed")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        &canary,
        fixture.root.join("installed/harness-ultragoal.hugpkg"),
    )
    .unwrap();
    let failure = operation.apply(&empty, &plan).unwrap_err();
    assert_eq!(fs::read(&canary).unwrap(), b"outside\n");
    assert!(!format!("{failure:?}").contains("SECRET_PLUGIN_ADAPTER_CANARY"));
    fs::remove_file(canary).unwrap();
}

#[test]
fn ancestor_replacement_refuses_without_writing_the_substitute_root() {
    let fixture = Fixture::new("ancestor-swap");
    let bundle = fixture.bundle("0.0.11");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let mut operation = fixture.operation(&bundle, &plan);
    let displaced = fixture.root.with_file_name(format!(
        "{}-displaced",
        fixture.root.file_name().unwrap().to_string_lossy()
    ));
    fs::rename(&fixture.root, &displaced).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    fs::write(fixture.root.join("canary"), b"substitute\n").unwrap();
    assert!(operation.apply(&empty, &plan).is_err());
    assert_eq!(
        fs::read(fixture.root.join("canary")).unwrap(),
        b"substitute\n"
    );
    fs::remove_dir_all(&fixture.root).unwrap();
    fs::rename(&displaced, &fixture.root).unwrap();
}

#[test]
fn oversized_and_special_installed_objects_refuse_before_adapter_effects() {
    for kind in ["oversized", "special"] {
        let fixture = Fixture::new(kind);
        let bundle = fixture.bundle("0.0.11");
        let empty = LifecycleState::default();
        let plan = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let mut operation = fixture.operation(&bundle, &plan);
        fs::create_dir(fixture.root.join("installed")).unwrap();
        let target = fixture.root.join("installed/harness-ultragoal.hugpkg");
        if kind == "oversized" {
            let file = fs::File::create(&target).unwrap();
            file.set_len(65 * 1024 * 1024 + 1).unwrap();
        } else {
            #[cfg(unix)]
            {
                let listener = std::os::unix::net::UnixListener::bind(&target).unwrap();
                assert!(operation.apply(&empty, &plan).is_err());
                drop(listener);
                continue;
            }
        }
        assert!(operation.apply(&empty, &plan).is_err());
    }
}
