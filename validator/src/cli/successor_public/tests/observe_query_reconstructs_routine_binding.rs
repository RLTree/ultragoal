use super::*;
use std::process::Command;

#[test]
fn observe_query_reconstructs_the_current_routine_binding() {
    let repo = Repository::new("observe-routine-binding");
    fs::create_dir_all(repo.root.join("config")).unwrap();
    fs::create_dir_all(repo.root.join("src")).unwrap();
    fs::write(
        repo.root.join("config/routine-public.json"),
        include_bytes!("../../../../../templates/config/routine-public.json"),
    )
    .unwrap();
    fs::write(
        repo.root.join("config/routines.json"),
        include_bytes!("../../../../../templates/config/routines.json"),
    )
    .unwrap();
    fs::write(
        repo.root.join("src/lib.rs"),
        b"pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    git(&repo.root, &["add", "config", "src/lib.rs"]);
    git(&repo.root, &["commit", "-qm", "add routine configuration"]);

    let read_context = read_context(&repo.root).unwrap();
    let routine_context =
        super::super::routine::current_observability_context(&repo.root, &read_context)
            .unwrap()
            .unwrap();
    assert_ne!(routine_context.context_id(), read_context.context_id());
    fs::create_dir_all(repo.root.join("validation_artifacts/observability/spool")).unwrap();
    let store = EventStore::for_context(
        super::super::observe::store_path(&repo.root, &routine_context, "successor-runtime")
            .unwrap(),
        &routine_context,
        "successor-runtime",
    )
    .unwrap();
    let event = SemanticEvent::for_context(
        &routine_context,
        "successor-runtime",
        "routine-terminal",
        1,
        1,
        "check.routine.terminal",
        "pass",
    )
    .unwrap();
    assert!(store.append(&event).unwrap());

    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "observe", "query"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["context_id"], routine_context.context_id());
    assert_eq!(value["event_count"], 1);
    assert_eq!(value["events"][0]["event_id"], "routine-terminal");
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}
