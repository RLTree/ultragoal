use crate::distribution::{
    AppRegistryVerdict, DiscoveryVerdict, DistributionErrorId as ErrorId, ExpectedPrior,
    ExpectedTree, HostCapabilityDeclaration, InstallPlan, InstallScope, JourneyBinding, ScopedFile,
    ScopedInstall, ScopedTree, install, materialize_package, observe_app_registry,
    observe_discovery, registry_document,
};
use crate::journey_support::{JourneyFixture, renamed, write_scoped};
use crate::support::digest;
use serde_json::{Value, json};
use std::fs;

#[test]
fn wrong_scope_identity_duplicates_and_registered_hidden_fail_closed() {
    let fixture = JourneyFixture::new("registry-adversarial");
    let package = fixture.build("packages/current.hugpkg");
    let executable = crate::runtime_session::program();
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding = JourneyBinding::new(package.identity().clone(), &host).unwrap();
    let valid = registry_document(&binding, true, true).unwrap();
    let value: Value = serde_json::from_slice(&valid).unwrap();
    for (pointer, replacement) in [
        (
            "/context_id",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/candidate_id",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/home_id",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/project_id",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/host_id",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/capability_sha256",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        ("/entries/0/version", json!("0.0.10")),
        (
            "/entries/0/package_sha256",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
        (
            "/entries/0/installed_tree_sha256",
            json!("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            observe_discovery(
                Some(&serde_json::to_vec(&changed).unwrap()),
                &binding,
                &host,
            )
            .is_err(),
            "{pointer}"
        );
    }

    let hidden = registry_document(&binding, true, false).unwrap();
    assert_eq!(
        observe_app_registry(Some(&hidden), &binding, &host)
            .unwrap()
            .verdict(),
        AppRegistryVerdict::Verified,
    );
    let discovery = observe_discovery(Some(&hidden), &binding, &host).unwrap();
    assert_eq!(
        discovery.discovery_verdict(),
        DiscoveryVerdict::RegisteredHidden
    );
    assert!(!discovery.is_current_visible());

    let mut duplicate = value;
    let mut row = duplicate["entries"][0].clone();
    row["version"] = json!("0.0.10");
    duplicate["entries"].as_array_mut().unwrap().push(row);
    assert_eq!(
        observe_discovery(
            Some(&serde_json::to_vec(&duplicate).unwrap()),
            &binding,
            &host,
        )
        .unwrap_err()
        .id(),
        ErrorId::InstallConflict,
    );
}

#[test]
fn unsupported_app_surfaces_cannot_be_promoted_by_supplied_bytes() {
    let fixture = JourneyFixture::new("unsupported-app");
    let package = fixture.build("packages/current.hugpkg");
    let host = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.root,
        &fixture.project,
        "codex-app-api-unavailable",
    )
    .unwrap();
    let binding = JourneyBinding::new(package.identity().clone(), &host).unwrap();
    let registry = registry_document(&binding, true, true).unwrap();
    assert_eq!(
        observe_app_registry(None, &binding, &host)
            .unwrap()
            .verdict(),
        AppRegistryVerdict::Unsupported,
    );
    assert_eq!(
        observe_discovery(None, &binding, &host)
            .unwrap()
            .discovery_verdict(),
        DiscoveryVerdict::Unsupported,
    );
    assert_eq!(
        observe_discovery(Some(&registry), &binding, &host)
            .unwrap_err()
            .id(),
        ErrorId::CapabilityMismatch,
    );
}

#[test]
fn same_size_substitution_partial_install_and_renamed_root_preserve_external_state() {
    let fixture = JourneyFixture::new("filesystem-substitution");
    let package = fixture.build("packages/current.hugpkg");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"AAAA");
    let expected = digest(b"AAAA");
    fs::write(fixture.root.join("state/value.bin"), b"BBBB").unwrap();
    assert!(!file.apply(Some(&expected), Some(b"CCCC")).unwrap());
    assert_eq!(file.inspect(16).unwrap().unwrap(), b"BBBB");

    write_scoped(
        fixture.confined(),
        "plugins/harness-ultragoal.hugpkg",
        b"partial",
    );
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::Personal,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    assert_eq!(
        install(&plan, &package, &mut ScopedInstall::new(fixture.confined()))
            .unwrap_err()
            .id(),
        ErrorId::InstallConflict,
    );
    assert_eq!(
        ScopedFile::new(fixture.confined(), "plugins/harness-ultragoal.hugpkg")
            .unwrap()
            .inspect(64)
            .unwrap()
            .unwrap(),
        b"partial",
    );

    let root = fixture.confined();
    let moved = renamed(&fixture.root, "renamed");
    fs::rename(&fixture.root, &moved).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    assert_eq!(
        ScopedFile::new(root, "state/value.bin").unwrap_err().id(),
        ErrorId::ObjectChanged,
    );
    fs::remove_dir(&fixture.root).unwrap();
    fs::rename(&moved, &fixture.root).unwrap();
}

#[cfg(unix)]
#[test]
fn scoped_reads_reject_symlink_hardlink_and_special_file_substitution() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("filesystem-objects");
    write_scoped(fixture.confined(), "objects/source", b"safe");
    symlink("source", fixture.root.join("objects/link")).unwrap();
    fs::hard_link(
        fixture.root.join("objects/source"),
        fixture.root.join("objects/hard"),
    )
    .unwrap();
    let fifo_path = fixture.root.join("objects/fifo");
    let fifo = CString::new(fifo_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    for path in [
        "objects/link",
        "objects/source",
        "objects/hard",
        "objects/fifo",
    ] {
        assert_eq!(
            ScopedFile::new(fixture.confined(), path)
                .unwrap()
                .inspect(64)
                .unwrap_err()
                .id(),
            ErrorId::UnsafeObject,
            "{path}",
        );
    }
}

#[test]
fn interrupted_tree_materialization_requires_explicit_recovery() {
    let fixture = JourneyFixture::new("tree-recovery");
    let plan = fixture.plan();
    let target = "installed/tree";
    let mut tree = ScopedTree::new(fixture.confined(), target).unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut tree).unwrap();
    let token = &digest(target.as_bytes())[7..];
    let backup = fixture
        .root
        .join(format!(".hul-tree-{token}-backup-manual"));
    fs::rename(fixture.root.join(target), &backup).unwrap();
    assert!(tree.recover_interrupted().unwrap());
    assert_eq!(
        tree.inspect(4096, 64 * 1024 * 1024).unwrap().unwrap().len(),
        3
    );

    let stage = fixture.root.join(format!(".hul-tree-{token}-stage-manual"));
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("partial"), b"partial").unwrap();
    assert_eq!(
        materialize_package(
            &plan,
            &ExpectedTree::ExactDigest(plan.source_tree_sha256().into()),
            &mut tree,
        )
        .unwrap_err()
        .id(),
        ErrorId::EffectFailed,
    );
    assert!(stage.exists());
    assert!(tree.recover_interrupted().unwrap());
    assert!(!stage.exists());
}
