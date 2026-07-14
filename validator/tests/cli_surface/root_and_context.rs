use super::scenario::*;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

#[cfg(unix)]
#[test]
fn public_context_never_echoes_the_operator_supplied_absolute_root() {
    let repository = Repository::new("context-root-redaction");
    let before = observe(&repository.root);
    let output = repository.run(&["--json", "inspect", "context"]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(repository.root.to_str().unwrap()));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], "HarnessPublicContext-v1");
    assert!(value["roots"]["repository_root_id"].as_str().is_some());
    assert!(value["roots"]["worktree_root_id"].as_str().is_some());
    assert!(value["roots"].get("repository_root").is_none());
    assert!(value["roots"].get("worktree_root").is_none());
    assert_eq!(observe(&repository.root), before);
}

#[test]
fn malformed_root_options_use_the_typed_machine_contract() {
    let binary = env!("CARGO_BIN_EXE_ultragoal");
    for args in [
        &["--json", "--root"][..],
        &[
            "--json",
            "--root",
            "private-root-one",
            "--root",
            "private-root-two",
            "inspect",
        ][..],
    ] {
        let output = Command::new(binary).args(args).output().expect("CLI runs");
        assert_machine_error(&output);
        let bytes = emitted(&output);
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("private-root-one"));
        assert!(!text.contains("private-root-two"));
    }
}

#[cfg(unix)]
#[test]
fn trailing_malformed_root_is_ignored_after_the_public_help_boundary() {
    let repository = Repository::new("help-root-stop-boundary");
    let before = observe(&repository.root);
    let output = repository.run(["--help", "--root"]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("Harness Ultragoal successor CLI\n")
    );
    assert_eq!(observe(&repository.root), before);
}

#[cfg(unix)]
#[test]
fn non_utf8_root_is_a_typed_zero_write_error_even_when_json_follows() {
    let repository = Repository::new("non-utf8-root");
    let before = observe(&repository.root);
    let output = Command::new(env!("CARGO_BIN_EXE_ultragoal"))
        .current_dir(&repository.root)
        .arg("--root")
        .arg(std::ffi::OsString::from_vec(vec![0xff]))
        .arg("inspect")
        .arg("--json")
        .output()
        .expect("CLI runs");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(
        output.stderr,
        b"{\"schema_version\":\"harness-ultragoal.cli-error.v1\",\"error_id\":\"CLI_NON_UTF8_ARGUMENT\",\"exit_code\":2}\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(observe(&repository.root), before);
}
