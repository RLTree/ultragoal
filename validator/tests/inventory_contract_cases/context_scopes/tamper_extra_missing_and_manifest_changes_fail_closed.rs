#[test]
fn tamper_extra_missing_and_manifest_changes_fail_closed() {
    for (label, mutation) in [
        ("tamper", "tamper"),
        ("extra", "extra"),
        ("missing", "missing"),
        ("manifest", "manifest"),
    ] {
        let repo = exact_repo(label);
        match mutation {
            "tamper" => repo.write(
                &format!("{CANDIDATE_ROOT}/00-READ-ME-FIRST.md"),
                b"tampered",
            ),
            "extra" => repo.write(&format!("{CANDIDATE_ROOT}/EXTRA.md"), b"extra"),
            "missing" => {
                fs::remove_file(repo.root.join(CANDIDATE_ROOT).join("00-READ-ME-FIRST.md")).unwrap()
            }
            "manifest" => repo.write(
                &format!("{CANDIDATE_ROOT}/ZIP_INCLUDE_MANIFEST.json"),
                b"{}",
            ),
            _ => unreachable!(),
        }
        repo.commit();
        assert_fallback(&repo, "non_authoritative_context_verification_failed");
    }
}

#[test]
fn unlisted_empty_directory_fails_closed() {
    let repo = exact_repo("context-extra-empty-directory");
    repo.commit();
    fs::create_dir(repo.root.join(CANDIDATE_ROOT).join("UNLISTED-EMPTY"))
        .expect("unlisted empty directory");
    assert_fallback(&repo, "non_authoritative_context_verification_failed");
}

#[test]
fn prose_fence_comment_and_json_bait_cannot_self_classify() {
    let repo = TestRepo::new("context-bait");
    repo.write(
        &format!("{CANDIDATE_ROOT}/00-READ-ME-FIRST.md"),
        br#"authority.binding=false
```json
{"status":"candidate_for_independent_review","active_contract_replaced":false}
```
<!-- non-authoritative context-only -->"#,
    );
    repo.write(REGISTRY, b"{}");
    repo.commit();
    assert_fallback(&repo, "invalid_non_authoritative_context_registry");
}

#[cfg(unix)]
#[test]
fn symlinked_candidate_file_fails_closed() {
    let repo = exact_repo("context-symlink");
    let target = repo.root.join(CANDIDATE_ROOT).join("00-READ-ME-FIRST.md");
    fs::remove_file(&target).unwrap();
    std::os::unix::fs::symlink("01-CURRENT-SYSTEM-ASSESSMENT.md", &target).unwrap();
    repo.commit();
    assert_fallback(&repo, "non_authoritative_context_verification_failed");
}

#[cfg(unix)]
#[test]
fn special_candidate_entry_fails_closed() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = exact_repo("context-special");
    repo.commit();
    let special = repo.root.join(CANDIDATE_ROOT).join("SPECIAL");
    let name = CString::new(special.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_fallback(&repo, "non_authoritative_context_verification_failed");
}
