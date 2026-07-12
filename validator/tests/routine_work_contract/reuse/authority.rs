use super::super::capture::{
    CommandSpec, PublicArtifact, capture_public_for_test, reset_test_file_open_attempts,
    set_test_artifact_pause_ms, test_artifact_is_paused, test_file_open_attempts,
};
use super::super::routine_work::{
    ReuseDecision, RoutineErrorId, assess_reuse, capture_executed_result, observe_result_artifact,
    set_test_live_authority_hook,
};
use super::super::support::{TempRepo, sha};
use super::common::{
    authority_plan, capture_guard, capture_race_guard, capture_run, expectation, result_bytes,
    syntax_evidence,
};
use std::fs;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
#[test]
fn real_capture_run_and_anchored_result_issue_opaque_executed_work() {
    let _capture = capture_guard();
    let repo = TempRepo::new("executed-authority");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let bytes = result_bytes(&context, &expectation, b"syntax");
    let run = capture_run(&repo, &context, &expectation, &bytes);
    let execution = capture_executed_result(&context, &expectation, &run).unwrap();
    assert_eq!(execution.work().node_id(), "syntax");
    assert!(execution.work().behavior_observed());
    assert!(execution.work().dependency_result(&context).is_ok());
    assert!(execution.receipt_json().starts_with(b"{"));
}

#[cfg(target_os = "macos")]
#[test]
fn executed_capture_revalidates_after_initial_live_authority() {
    let _capture = capture_guard();
    let repo = TempRepo::new("executed-live-mutation");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let bytes = result_bytes(&context, &expectation, b"syntax");
    let run = capture_run(&repo, &context, &expectation, &bytes);
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        fs::write(
            root.join("src/lib.rs"),
            b"mutated during executed capture\n",
        )
        .unwrap();
    });
    assert_eq!(
        capture_executed_result(&context, &expectation, &run)
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
}

#[cfg(target_os = "macos")]
#[test]
fn result_observation_revalidates_after_initial_live_authority() {
    let _capture = capture_guard();
    let repo = TempRepo::new("observation-live-mutation");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let bytes = result_bytes(&context, &expectation, b"syntax");
    let run = capture_run(&repo, &context, &expectation, &bytes);
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        fs::write(root.join("src/lib.rs"), b"mutated during observation\n").unwrap();
    });
    assert_eq!(
        observe_result_artifact(&context, &expectation, &run.artifacts()[0])
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
}

#[cfg(target_os = "macos")]
#[test]
fn missing_artifact_wrong_command_and_wrong_tool_program_fail_closed() {
    let _capture = capture_guard();
    let repo = TempRepo::new("executed-missing");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let missing = CommandSpec::catalog_read("syntax", "true")
        .run(&context)
        .unwrap();
    assert_eq!(
        capture_executed_result(&context, &expectation, &missing)
            .unwrap_err()
            .id(),
        RoutineErrorId::ObservationFailed
    );
    let bytes = result_bytes(&context, &expectation, b"syntax");
    repo.write("routine-cache/syntax.result.json", &bytes);
    let wrong_command = CommandSpec::catalog_read("different", "true")
        .public_artifact(PublicArtifact::new("routine-cache/syntax.result.json"))
        .run(&context)
        .unwrap();
    assert!(capture_executed_result(&context, &expectation, &wrong_command).is_err());
    let wrong_program = mutate_string(&bytes, "capture_program_sha256", &sha(b"other-program"));
    let run = capture_run(&repo, &context, &expectation, &wrong_program);
    assert!(capture_executed_result(&context, &expectation, &run).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn captured_result_rejects_wrong_plan_check_scope_context_root_config_tool_and_input() {
    let _capture = capture_guard();
    let repo = TempRepo::new("executed-binding");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let original = result_bytes(&context, &expectation, b"syntax");
    let cases = [
        ("context_id", sha(b"wrong-context")),
        ("candidate_id", sha(b"wrong-candidate")),
        ("root_id", sha(b"wrong-root")),
        ("configuration_id", sha(b"wrong-config")),
        ("graph_id", sha(b"wrong-graph")),
        ("plan_id", sha(b"wrong-plan")),
        ("node_id", "other".to_owned()),
        ("result_scope", "other".to_owned()),
        ("tool_identity", sha(b"wrong-tool")),
        ("input_id", sha(b"wrong-input")),
    ];
    for (field, replacement) in cases {
        let bytes = mutate_string(&original, field, &replacement);
        let run = capture_run(&repo, &context, &expectation, &bytes);
        assert!(
            capture_executed_result(&context, &expectation, &run).is_err(),
            "{field}"
        );
    }
}

#[cfg(all(target_os = "macos", unix))]
#[test]
fn special_result_artifacts_fail_before_evidence_minting() {
    let _capture = capture_guard();
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixListener;

    let repo = TempRepo::new("s");
    let (context, _) = authority_plan(&repo);
    repo.write("routine-cache/real", b"bytes");
    std::os::unix::fs::symlink("real", repo.root().join("routine-cache/link")).unwrap();
    fs::hard_link(
        repo.root().join("routine-cache/real"),
        repo.root().join("routine-cache/hard"),
    )
    .unwrap();
    let fifo = repo.root().join("routine-cache/fifo");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let _socket = UnixListener::bind(repo.root().join("routine-cache/socket")).unwrap();
    for path in ["link", "hard", "real", "fifo", "socket"] {
        assert!(
            capture_public_for_test(
                &context,
                vec![PublicArtifact::new(format!("routine-cache/{path}"))],
            )
            .is_err(),
            "{path}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn result_artifact_swap_has_initial_and_final_validation_without_retry() {
    let _capture = capture_race_guard();
    let repo = TempRepo::new("result-race");
    let (context, plan) = authority_plan(&repo);
    let expectation = expectation(&context, &plan, "syntax", Vec::new());
    let bytes = result_bytes(&context, &expectation, b"syntax");
    let path = repo.root().join("routine-cache/syntax.result.json");
    repo.write("routine-cache/syntax.result.json", &bytes);
    reset_test_file_open_attempts();
    set_test_artifact_pause_ms(100);
    let swapper = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !test_artifact_is_paused() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        fs::rename(&path, path.with_extension("original")).unwrap();
        fs::write(&path, vec![b'x'; bytes.len()]).unwrap();
    });
    let error = capture_public_for_test(
        &context,
        vec![PublicArtifact::new("routine-cache/syntax.result.json")],
    )
    .unwrap_err();
    swapper.join().unwrap();
    assert!(error.contains("identity changed") || error.contains("content changed"));
    assert_eq!(test_file_open_attempts(), 2);
}

#[cfg(target_os = "macos")]
#[test]
fn reuse_assessment_is_recursively_zero_write() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-zero-write");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    let tree = repo.tree();
    let status = repo.status();
    assert!(matches!(
        assess_reuse(&context, &expectation, &receipt, &observed).unwrap(),
        ReuseDecision::Hit(_)
    ));
    assert_eq!(repo.tree(), tree);
    assert_eq!(repo.status(), status);
}

fn mutate_string(bytes: &[u8], field: &str, replacement: &str) -> Vec<u8> {
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let current = value
        .get(field)
        .or_else(|| value["binding"].get(field))
        .and_then(serde_json::Value::as_str)
        .unwrap();
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .replacen(
            &format!("\"{field}\":\"{current}\""),
            &format!("\"{field}\":\"{replacement}\""),
            1,
        )
        .into_bytes()
}
