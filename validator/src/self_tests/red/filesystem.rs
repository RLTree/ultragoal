use serde_json::json;
use std::path::Path;

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

#[test]
fn red_filesystem_fixtures_cover_file_symlink_and_path_failures() {
    let root = crate::self_tests::boundaries::support::temp_root("red-filesystem");
    assert!(crate::red::filesystem::fixtures::materialize(&root, &json!({})).is_ok());
    for (packet, expected) in [
        (
            json!({"filesystem_fixtures":[{"kind":"unknown"}]}),
            "red_filesystem_fixture_kind_unknown",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"file"}]}),
            "red_filesystem_fixture_path_missing",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"file","path":"../x","contents":"x"}]}),
            "red_filesystem_fixture_path_invalid",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"file","path":"/tmp/x","contents":"x"}]}),
            "red_filesystem_fixture_path_invalid",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"file","path":"dir/new.txt"}]}),
            "red_filesystem_fixture_contents_missing",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"file","path":"missing/x","contents":"x"}]}),
            "red_filesystem_fixture_parent_missing",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"symlink","target":"target.txt"}]}),
            "red_filesystem_fixture_path_missing",
        ),
        (
            json!({"filesystem_fixtures":[{"kind":"symlink","path":"link"}]}),
            "red_filesystem_fixture_target_missing",
        ),
    ] {
        let err = crate::red::filesystem::fixtures::materialize(&root, &packet)
            .expect_err("fixture failure");
        assert!(err.contains(expected), "{expected}: {err}");
    }
    std::fs::create_dir_all(root.join("dir")).expect("dir");
    write_text(&root.join("dir/existing.txt"), "exists");
    let err = crate::red::filesystem::fixtures::materialize(
        &root,
        &json!({"filesystem_fixtures":[{"kind":"file","path":"dir/existing.txt","contents":"x"}]}),
    )
    .expect_err("existing path");
    assert!(err.contains("red_filesystem_fixture_path_exists"));
    let err = crate::red::filesystem::fixtures::materialize(
        &root,
        &json!({"filesystem_fixtures":[{"kind":"symlink","path":"dir/existing.txt","target":"existing.txt"}]}),
    )
    .expect_err("existing symlink path");
    assert!(err.contains("red_filesystem_fixture_path_exists"));
    let err = crate::red::filesystem::fixtures::materialize(
            &root,
            &json!({"filesystem_fixtures":[{"kind":"symlink","path":"dir/link.txt","target":"missing.txt"}]}),
        )
        .expect_err("missing symlink target");
    assert!(err.contains("red_filesystem_fixture_target_invalid"));
    let missing_root = root.join("missing-root");
    let err = crate::red::filesystem::fixtures::materialize(
            &missing_root,
            &json!({"filesystem_fixtures":[{"kind":"symlink","path":"link.txt","target":root.join("dir/existing.txt").to_string_lossy()}]}),
        )
        .expect_err("invalid root");
    assert!(err.contains("root invalid"), "{err}");
    {
        let _guard = crate::red::filesystem::fixtures::materialize(
            &root,
            &json!({"filesystem_fixtures":[{"kind":"file","path":"dir/new.txt","contents":"new"}]}),
        )
        .expect("file fixture");
        assert!(root.join("dir/new.txt").is_file());
    }
    assert!(!root.join("dir/new.txt").exists());

    #[cfg(unix)]
    {
        let outside = root
            .parent()
            .expect("root parent")
            .join("red-filesystem-outside");
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).expect("outside dir");
        std::fs::write(outside.join("outside.txt"), "outside").expect("outside");
        let err = crate::red::filesystem::fixtures::materialize(
            &root,
            &json!({"filesystem_fixtures":[{"kind":"symlink","path":"dir/escape.txt","target":"../../red-filesystem-outside/outside.txt"}]}),
        )
        .expect_err("escaping symlink target");
        assert!(err.contains("red_filesystem_fixture_target_escapes_root"));

        {
            let _guard = crate::red::filesystem::fixtures::materialize(
                &root,
                &json!({"filesystem_fixtures":[{"kind":"symlink","path":"dir/link.txt","target":"existing.txt"}]}),
            )
            .expect("symlink fixture");
            assert!(std::fs::symlink_metadata(root.join("dir/link.txt")).is_ok());
        }
        assert!(!root.join("dir/link.txt").exists());
        std::fs::remove_dir_all(outside).expect("cleanup outside");
    }
    std::fs::remove_dir_all(root).expect("cleanup red filesystem");
}

#[cfg(unix)]
#[test]
fn red_filesystem_reports_permission_denied_write_and_symlink_failures() {
    use std::os::unix::fs::PermissionsExt;

    let root = crate::self_tests::boundaries::support::temp_root("red-filesystem-denied");
    std::fs::create_dir_all(root.join("locked")).expect("locked dir");
    std::fs::write(root.join("target.txt"), "target").expect("target");
    let locked = root.join("locked");
    let original = std::fs::metadata(&locked)
        .expect("locked metadata")
        .permissions()
        .mode();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555))
        .expect("lock permissions");

    let file_result = crate::red::filesystem::fixtures::materialize(
        &root,
        &json!({"filesystem_fixtures":[{"kind":"file","path":"locked/new.txt","contents":"x"}]}),
    );
    let symlink_result = crate::red::filesystem::fixtures::materialize(
        &root,
        &json!({"filesystem_fixtures":[{"kind":"symlink","path":"locked/link.txt","target":"../target.txt"}]}),
    );

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(original))
        .expect("restore permissions");
    let file_err = file_result.expect_err("locked file write fails");
    assert!(
        file_err.contains("red_filesystem_fixture_file_write_failed"),
        "{file_err}"
    );
    let symlink_err = symlink_result.expect_err("locked symlink fails");
    assert!(
        symlink_err.contains("red_filesystem_fixture_symlink_failed"),
        "{symlink_err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup locked red filesystem");
}
