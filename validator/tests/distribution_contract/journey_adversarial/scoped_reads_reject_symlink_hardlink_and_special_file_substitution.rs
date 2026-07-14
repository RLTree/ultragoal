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
