use serde_json::json;
use std::fs;
use std::path::Path;

#[cfg(unix)]
#[test]
fn output_path_rejects_symlinks_and_preserves_existing_files() {
    let base = crate::self_tests::boundaries::support::temp_root("output-path-unix");
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
    let base = crate::self_tests::boundaries::support::temp_root("output-path-hardlink");
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

    let base = crate::self_tests::boundaries::support::temp_root("output-path-rename");
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
fn skill_links_cover_missing_invalid_local_and_markdown_references() {
    let root = crate::self_tests::boundaries::support::temp_root("skill-links");
    fs::create_dir_all(root.join("skills/demo")).expect("skill dir");
    fs::write(root.join("root-ref.md"), "root").expect("root ref");
    fs::write(root.join("skills/demo/local.md"), "local").expect("local ref");
    fs::write(
        root.join("skills/demo/SKILL.md"),
        "`local.md`\n[root](root-ref.md)\n[broken](unterminated",
    )
    .expect("skill");
    fs::write(root.join("skills/demo/BAD.md"), [0xff, 0xfe]).expect("bad utf8");
    let manifest = json!({
        "skills": [
            {"path":"skills/demo/SKILL.md"},
            {"path":"skills/demo/missing.md"},
            {"path":"skills/demo/BAD.md"}
        ]
    });
    let failures = crate::skill_links::manifest_failures(&root, &manifest);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, "skill_local_reference_missing");
    assert!(failures[0].detail.contains("root-ref.md"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_artifact_refs_reject_boundary_substitutes() {
    let root = crate::self_tests::boundaries::support::temp_root("artifact-refs");
    fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    fs::write(root.join("artifacts/proof.json"), "{}").expect("proof");
    let digest = crate::digest::file(&root.join("artifacts/proof.json")).expect("digest");
    crate::package::artifact::refs::validate_path_digest(
        &root,
        "artifacts/proof.json",
        &digest,
        "proof",
    )
    .expect("valid proof artifact");
    for (path, got, expected) in [
        ("", &digest, "lacks non-zero"),
        ("artifacts/fixture-proof.json", &digest, "placeholder"),
        ("/tmp/proof.json", &digest, "absolute"),
        ("../proof.json", &digest, "escapes"),
        ("artifacts/missing.json", &digest, "artifact missing"),
        ("artifacts", &digest, "not a regular file"),
        (
            "artifacts/proof.json",
            &crate::self_tests::boundaries::support::sha('9'),
            "digest mismatch",
        ),
    ] {
        let err = crate::package::artifact::refs::validate_path_digest(&root, path, got, "proof")
            .expect_err("invalid artifact rejected");
        assert!(err.contains(expected), "{path}: {err}");
    }
    let object = json!({"path":"artifacts/proof.json","digest":digest});
    crate::package::artifact::refs::validate_object(&root, &object, "object").expect("object");
    let command =
        json!({"artifact_path":"artifacts/proof.json","artifact_digest":object["digest"]});
    crate::package::artifact::refs::validate_command_artifact(&root, &command, "command")
        .expect("command artifact");
    #[cfg(unix)]
    {
        let link = root.join("artifacts/link.json");
        std::os::unix::fs::symlink(root.join("artifacts/proof.json"), &link).expect("link");
        let err = crate::package::artifact::refs::validate_path_digest(
            &root,
            "artifacts/link.json",
            &digest,
            "proof",
        )
        .expect_err("symlink artifact rejected");
        assert!(err.contains("symlink"), "{err}");

        let source = root.join("artifacts/hard-source.json");
        let hard = root.join("artifacts/hard.json");
        fs::write(&source, "{}").expect("hard source");
        fs::hard_link(&source, &hard).expect("hard link");
        let err = crate::package::artifact::refs::validate_path_digest(
            &root,
            "artifacts/hard.json",
            &crate::self_tests::boundaries::support::sha('b'),
            "proof",
        )
        .expect_err("hard-linked artifact rejected");
        assert!(err.contains("artifact unreadable"), "{err}");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_resource_purpose_rejects_invalid_active_fixture_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("resource-purpose-invalid");
    fs::create_dir_all(&root).expect("resource root");
    let failures = crate::package::resource::purpose::failures(
        &root,
        &json!({"resources":["artifacts/stale-proof.json"]}),
    );
    assert!(failures.iter().all(|failure| {
        failure.code != crate::package::resource::purpose::FIXTURE_SUPPORT_ACTIVE_ARTIFACT
    }));
    assert!(failures.iter().any(|failure| {
        failure.code == crate::package::resource::purpose::STALE_ARTIFACT_RESOURCE
    }));
    let _ = fs::remove_dir_all(root);
}
