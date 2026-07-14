#[cfg(unix)]
#[test]
fn package_inputs_reject_symlink_hardlink_and_special_files() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let symlinked = Fixture::complete("package-source-symlink");
    write_sources(&symlinked);
    fs::remove_file(symlinked.root.join("source/skill-one.md")).unwrap();
    symlink("plugin.json", symlinked.root.join("source/skill-one.md")).unwrap();
    assert_eq!(
        plan_package(&symlinked.root, &spec(false))
            .unwrap_err()
            .id(),
        ErrorId::UnsafeObject
    );

    let linked = Fixture::complete("package-source-hardlink");
    write_sources(&linked);
    fs::hard_link(
        linked.root.join("source/skill-one.md"),
        linked.root.join("source/skill-alias.md"),
    )
    .unwrap();
    assert_eq!(
        plan_package(&linked.root, &spec(false)).unwrap_err().id(),
        ErrorId::UnsafeObject
    );

    let fifo = Fixture::complete("package-source-fifo");
    write_sources(&fifo);
    fs::remove_file(fifo.root.join("source/skill-one.md")).unwrap();
    let path = fifo.root.join("source/skill-one.md");
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(
        plan_package(&fifo.root, &spec(false)).unwrap_err().id(),
        ErrorId::UnsafeObject
    );
}

#[test]
fn malformed_duplicate_unsafe_and_oversized_inputs_are_rejected() {
    let fixture = Fixture::complete("package-invalid");
    write_sources(&fixture);
    let mut duplicate: Value = serde_json::from_slice(&spec(false)).unwrap();
    duplicate["entries"][1]["path"] = duplicate["entries"][0]["path"].clone();
    assert_eq!(
        plan_package(&fixture.root, &serde_json::to_vec(&duplicate).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::InvalidSpec
    );
    let mut escape: Value = serde_json::from_slice(&spec(false)).unwrap();
    escape["entries"][0]["source_path"] = json!("../canary");
    let error = plan_package(&fixture.root, &serde_json::to_vec(&escape).unwrap()).unwrap_err();
    assert_eq!(error.id(), ErrorId::InvalidPath);
    assert!(!error.to_string().contains("canary"));
    fs::OpenOptions::new()
        .write(true)
        .open(fixture.root.join("source/skill-one.md"))
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        plan_package(&fixture.root, &spec(false)).unwrap_err().id(),
        ErrorId::ObjectTooLarge
    );

    let wrong_manifest = Fixture::complete("package-wrong-manifest-version");
    write_sources(&wrong_manifest);
    let mut wrong = manifest();
    wrong["version"] = json!("0.0.10");
    fs::write(
        wrong_manifest.root.join("source/plugin.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert_eq!(
        plan_package(&wrong_manifest.root, &spec(false))
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
}
