use serde_json::json;
use std::fs;

#[test]
fn skill_links_cover_missing_invalid_local_and_markdown_references() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("skill-links");
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
    assert!(failures[0].detail.contains("root-ref.md"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_artifact_refs_reject_boundary_substitutes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("artifact-refs");
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
            &crate::self_tests::boundaries::workspace_fixtures::sha('9'),
            "digest mismatch",
        ),
    ] {
        let err = crate::package::artifact::refs::validate_path_digest(&root, path, got, "proof")
            .expect_err("invalid artifact rejected");
        assert!(err.contains(expected), "{path}: {err}");
    }
    let object = json!({"path":"artifacts/proof.json","digest":digest});
    crate::package::artifact::refs::ArtifactRef::from_object(&object, "object")
        .and_then(|artifact| artifact.validate(&root, "object"))
        .expect("object");
    let missing_digest = json!({"path":"artifacts/proof.json"});
    let err = crate::package::artifact::refs::ArtifactRef::from_object(&missing_digest, "object")
        .and_then(|artifact| artifact.validate(&root, "object"))
        .expect_err("object digest is required");
    assert!(err.contains("missing digest"), "{err}");
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
            &crate::self_tests::boundaries::workspace_fixtures::sha('b'),
            "proof",
        )
        .expect_err("hard-linked artifact rejected");
        assert!(err.contains("artifact unreadable"), "{err}");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_resource_purpose_rejects_invalid_active_fixture_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("resource-purpose-invalid");
    fs::create_dir_all(&root).expect("resource root");
    let failures = crate::package::resource::purpose::failures(
        &root,
        &json!({"resources":["artifacts/stale-proof.json"]}),
    );
    assert!(failures.iter().all(|failure| {
        failure.code != crate::package::resource::purpose::FIXTURE_ONLY_ACTIVE_ARTIFACT
    }));
    assert!(failures.iter().any(|failure| {
        failure.code == crate::package::resource::purpose::STALE_ARTIFACT_RESOURCE
    }));
    let _ = fs::remove_dir_all(root);
}
