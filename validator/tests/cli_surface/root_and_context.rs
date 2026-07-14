use super::scenario::*;
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
