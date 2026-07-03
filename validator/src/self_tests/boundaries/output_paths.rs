use std::fs;
use std::path::Path;

#[cfg(unix)]
#[test]
fn output_path_rejects_symlinks_and_preserves_existing_files() {
    let base = crate::self_tests::boundaries::workspace_fixtures::temp_root("output-path-unix");
    let _ = fs::remove_dir_all(&base);
    let real = base.join("real");
    let link = base.join("link");
    fs::create_dir_all(&real).expect("create real dir");
    std::os::unix::fs::symlink(&real, &link).expect("create symlink");
    let target = link.join("nested").join("receipt.json");

    let err = crate::output_path::prepare_parent(&target).expect_err("symlink parent rejected");
    assert!(err.contains("output path uses symlink"));
    assert!(!real.join("nested").exists());

    let real_file = base.join("real.json");
    let symlink_leaf = base.join("receipt.json");
    fs::write(&real_file, "{}").expect("write real file");
    std::os::unix::fs::symlink(&real_file, &symlink_leaf).expect("create leaf symlink");
    let err = crate::output_path::create_file(&symlink_leaf, "receipt")
        .expect_err("symlink leaf rejected");
    let symlink_rejected = err.contains("output path uses symlink");
    let create_rejected = err.contains("create failed");
    assert!(symlink_rejected || create_rejected, "{err}");

    let tmp = base.join("tmp.json");
    fs::write(&tmp, "new").expect("tmp");
    let err = crate::output_path::finish_temp_file(&tmp, &symlink_leaf, "receipt")
        .expect_err("symlink destination rejected");
    assert!(err.contains("output path uses symlink"));
    assert!(!tmp.exists());
    assert_eq!(fs::read_to_string(&real_file).expect("read real"), "{}");
    let _ = fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn output_path_atomic_write_does_not_truncate_hard_linked_destination() {
    let base = crate::self_tests::boundaries::workspace_fixtures::temp_root("output-path-hardlink");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");
    let outside = base.join("outside.json");
    let dest = base.join("receipt.json");
    fs::write(&outside, "outside").expect("write outside");
    fs::hard_link(&outside, &dest).expect("create hard link");

    crate::output_path::write(&dest, "new", "receipt").expect("atomic write");

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside"),
        "outside"
    );
    assert_eq!(fs::read_to_string(&dest).expect("read dest"), "new");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn output_path_rejects_unsafe_paths_and_cwd_failures() {
    assert!(
        crate::output_path::prepare_parent(Path::new(""))
            .expect_err("empty path rejected")
            .contains("output path is empty")
    );
    assert!(
        crate::output_path::prepare_parent(Path::new("../receipt.json"))
            .expect_err("parent segment rejected")
            .contains("parent segment")
    );
    assert!(
        crate::output_path::create_temp_file(Path::new("/"), "receipt")
            .expect_err("root output lacks file name")
            .contains("lacks file name")
    );
    assert!(
        crate::output_path::reject_existing_symlink_components(Path::new("../x"))
            .expect_err("parent segment rejected")
            .contains("unsafe output path")
    );
    assert!(
        crate::output_path::reject_symlink_components(Path::new("../x"))
            .expect_err("parent segment rejected")
            .contains("unsafe output path")
    );
    let cwd_err = crate::output_path::start_cursor_with(Path::new("relative.json"), || {
        Err(std::io::Error::other("forced cwd failure"))
    })
    .expect_err("cwd failure reported");
    assert!(cwd_err.contains("cwd unavailable"));
    assert!(
        crate::output_path::write_result(
            Path::new("receipt.json"),
            "receipt",
            Err(std::io::Error::other("forced write failure")),
        )
        .expect_err("write failure reported")
        .contains("receipt write failed")
    );
    assert!(
        crate::output_path::sync_result(
            Path::new(".receipt.tmp"),
            "receipt",
            Err(std::io::Error::other("forced sync failure")),
        )
        .expect_err("sync failure reported")
        .contains("receipt sync failed")
    );

    let base = crate::self_tests::boundaries::workspace_fixtures::temp_root("output-path-rename");
    fs::create_dir_all(&base).expect("base");
    let err = crate::output_path::finish_temp_file(
        &base.join("missing.tmp"),
        &base.join("out.json"),
        "receipt",
    )
    .expect_err("missing temp cannot be renamed");
    assert!(!err.is_empty());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn claim_artifact_path_rejects_external_claim_outputs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("claim-output-paths");
    fs::create_dir_all(&root).expect("root");

    let relative = crate::output_path::claim_artifact_path(
        &root,
        Path::new("validation_artifacts/observability/receipt.json"),
        "receipt",
    )
    .expect("root-relative claim artifact");
    assert_eq!(
        relative,
        root.join("validation_artifacts/observability/receipt.json")
    );

    let absolute = crate::output_path::claim_artifact_path(
        &root,
        &root.join("outside/receipt.json"),
        "receipt",
    )
    .expect_err("absolute output cannot support claims");
    assert!(absolute.contains("root-relative claim artifact path"));
    assert!(absolute.contains("external debug only"));

    let traversal =
        crate::output_path::claim_artifact_path(&root, Path::new("../receipt.json"), "receipt")
            .expect_err("parent traversal rejected");
    assert!(traversal.contains("must stay inside the package root"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn literal_claim_artifact_path_accepts_product_owned_constants() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("literal-claim-output");
    fs::create_dir_all(&root).expect("root");

    let path = crate::output_path::literal_claim_artifact_path(
        &root,
        "validation_artifacts/observability/static-receipt.json",
        "static receipt",
    );
    assert_eq!(
        path,
        root.join("validation_artifacts/observability/static-receipt.json")
    );

    let invalid = std::panic::catch_unwind(|| {
        crate::output_path::literal_claim_artifact_path(&root, "/tmp/receipt.json", "bad receipt");
    });
    assert!(invalid.is_err());

    let _ = fs::remove_dir_all(root);
}
