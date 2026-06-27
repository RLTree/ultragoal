use serde_json::json;

#[cfg(unix)]
#[test]
fn target_artifact_refs_reject_missing_placeholder_directory_symlink_and_bad_json() {
    let root = crate::self_tests::boundaries::support::temp_root("target-artifacts");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&artifacts).expect("artifacts");
    std::fs::write(artifacts.join("ok.json"), "{\"ok\":true}").expect("ok");
    std::fs::write(artifacts.join("bad.json"), "{").expect("bad");
    std::fs::write(artifacts.join("placeholder.json"), "{}").expect("placeholder");
    std::os::unix::fs::symlink(artifacts.join("ok.json"), artifacts.join("link.json"))
        .expect("symlink");
    let digest = crate::digest::file(&artifacts.join("ok.json")).expect("digest");
    let good = json!({"path":"artifacts/ok.json","digest":digest});
    assert!(
        crate::target_repo::artifact_refs::artifact_ref_error(&root, &good, "artifact").is_none()
    );
    assert_eq!(
        crate::target_repo::artifact_refs::read_artifact_json(&root, &good, "artifact")
            .expect("json")["ok"],
        true
    );

    for (item, expected) in [
        (json!({}), "lacks non-zero"),
        (
            json!({"path":"artifacts/placeholder.json","digest":crate::self_tests::boundaries::support::sha('1')}),
            "placeholder",
        ),
        (
            json!({"path":"artifacts","digest":crate::self_tests::boundaries::support::sha('1')}),
            "not a regular file",
        ),
        (
            json!({"path":"artifacts/link.json","digest":crate::self_tests::boundaries::support::sha('1')}),
            "symlink",
        ),
        (
            json!({"path":"artifacts/missing.json","digest":crate::self_tests::boundaries::support::sha('1')}),
            "missing",
        ),
        (
            json!({"path":"artifacts/ok.json","digest":crate::self_tests::boundaries::support::sha('1')}),
            "digest mismatch",
        ),
    ] {
        let err = crate::target_repo::artifact_refs::artifact_ref_error(&root, &item, "artifact")
            .expect("artifact rejected");
        assert!(err.contains(expected), "{expected}: {err}");
    }

    let bad = json!({
        "path":"artifacts/bad.json",
        "digest":crate::digest::file(&artifacts.join("bad.json")).expect("bad digest")
    });
    assert!(
        crate::target_repo::artifact_refs::read_artifact_json(&root, &bad, "artifact")
            .expect_err("bad json rejected")
            .contains("json malformed")
    );
    let hard_source = artifacts.join("hard-source.json");
    let hard_link = artifacts.join("hard.json");
    std::fs::write(&hard_source, "{\"hard\":true}").expect("hard source");
    std::fs::hard_link(&hard_source, &hard_link).expect("hard link");
    let hard = json!({
        "path":"artifacts/hard.json",
        "digest":crate::self_tests::boundaries::support::sha('h')
    });
    let err = crate::target_repo::artifact_refs::artifact_ref_error(&root, &hard, "artifact")
        .expect("hard-linked artifact rejected");
    assert!(err.contains("unreadable"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup target artifacts");
}
