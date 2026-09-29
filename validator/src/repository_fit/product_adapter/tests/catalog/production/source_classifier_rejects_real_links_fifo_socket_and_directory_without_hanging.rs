use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn production_source_classifier_rejects_real_links_fifo_socket_and_directory_without_hanging()
 {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let symlink_fixture = SourceFixture::new("symlink");
    symlink_fixture.write("real", b"bytes");
    symlink("real", symlink_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &symlink_fixture.templates,
            &symlink_fixture.output,
        )
        .is_err()
    );

    let hardlink_fixture = SourceFixture::new("hardlink");
    hardlink_fixture.write("real", b"bytes");
    fs::hard_link(
        hardlink_fixture.templates.join("real"),
        hardlink_fixture.templates.join("A.md"),
    )
    .unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &hardlink_fixture.templates,
            &hardlink_fixture.output,
        )
        .is_err()
    );

    let fifo_fixture = SourceFixture::new("fifo");
    let fifo = fifo_fixture.templates.join("A.md");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o644) }, 0);
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &fifo_fixture.templates,
            &fifo_fixture.output,
        )
        .is_err()
    );

    let socket_fixture = SourceFixture::new("socket");
    let _listener =
        std::os::unix::net::UnixListener::bind(socket_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &socket_fixture.templates,
            &socket_fixture.output,
        )
        .is_err()
    );

    let directory_fixture = SourceFixture::new("directory");
    fs::create_dir(directory_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &directory_fixture.templates,
            &directory_fixture.output,
        )
        .is_err()
    );
}

#[test]
pub(crate) fn production_source_classifier_rejects_unknown_duplicate_and_source_list_drift() {
    let fixture = SourceFixture::new("list-drift");
    fixture.write("A.md", b"A");
    for manifest_bytes in [
        manifest(&["templates/Unknown.md"]),
        manifest(&["templates/A.md", "templates/A.md"]),
    ] {
        assert!(
            production_sources::stage_manifest_sources(
                &manifest_bytes,
                &fixture.templates,
                &fixture.output,
            )
            .is_err()
        );
    }
    fixture.write("B.md", b"B");
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
}

#[test]
pub(crate) fn production_source_classifier_rejects_mutation_after_inspection_before_staging() {
    let fixture = SourceFixture::new("inspection-race");
    fixture.write("A.md", b"prior");
    let target = fixture.templates.join("A.md");
    let replacement = fixture.templates.join("replacement");
    let result = production_sources::stage_manifest_sources_with_hook(
        &manifest(&["templates/A.md"]),
        &fixture.templates,
        &fixture.output,
        || {
            fs::write(&replacement, b"later").unwrap();
            fs::rename(&replacement, &target).unwrap();
        },
    );
    assert!(result.is_err());
    assert!(!fixture.staging().join("0000.bin").exists());
}

#[test]
pub(crate) fn production_source_classifier_rejects_added_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-add", |fixture| {
        fixture.write("B.md", b"extra");
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_removed_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-remove", |fixture| {
        fs::remove_file(fixture.templates.join("A.md")).unwrap();
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_renamed_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-rename", |fixture| {
        fs::rename(
            fixture.templates.join("A.md"),
            fixture.templates.join("B.md"),
        )
        .unwrap();
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_casefold_extra_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-casefold", |fixture| {
        fixture.write("a.md", b"case alias");
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_nested_extra_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-nested", |fixture| {
        fixture.write("unrelated/B.md", b"nested extra");
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_directory_insertion_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-directory", |fixture| {
        fs::create_dir(fixture.templates.join("unrelated")).unwrap();
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_add_then_remove_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-add-remove", |fixture| {
        let transient = fixture.templates.join("B.md");
        fs::write(&transient, b"transient").unwrap();
        fs::remove_file(transient).unwrap();
    });
}

#[test]
pub(crate) fn production_source_classifier_rejects_mutate_then_restore_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-mutate-restore", |fixture| {
        let source = fixture.templates.join("A.md");
        fs::write(&source, b"changed").unwrap();
        fs::write(source, b"prior").unwrap();
    });
}
