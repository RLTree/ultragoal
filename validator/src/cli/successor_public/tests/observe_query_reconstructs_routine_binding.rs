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
        super::super::routine::current_observability_context(&repo.root, None, &read_context)
            .unwrap()
            .unwrap()
            .context;
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

#[test]
fn observe_query_reconstructs_a_confined_child_routine_binding() {
    let repo = Repository::new("observe-child-routine-binding");
    let child = repo.add_nested_repository("child");
    fs::create_dir_all(child.join("config")).unwrap();
    fs::create_dir_all(child.join("src")).unwrap();
    fs::write(
        child.join("config/routine-public.json"),
        include_bytes!("../../../../../templates/config/routine-public.json"),
    )
    .unwrap();
    fs::write(
        child.join("config/routines.json"),
        include_bytes!("../../../../../templates/config/routines.json"),
    )
    .unwrap();
    fs::write(child.join("src/lib.rs"), b"pub fn value() -> u8 { 2 }\n").unwrap();
    git(&child, &["add", "config", "src/lib.rs"]);
    git(
        &child,
        &["commit", "-qm", "add child routine configuration"],
    );

    let read_context = read_context(&repo.root).unwrap();
    let binding = super::super::routine::current_observability_context(
        &repo.root,
        Some("child"),
        &read_context,
    )
    .unwrap()
    .unwrap();
    assert_eq!(binding.target, child);
    fs::create_dir_all(child.join("validation_artifacts/observability/spool")).unwrap();
    let store = EventStore::for_context(
        super::super::observe::store_path(&child, &binding.context, "successor-runtime").unwrap(),
        &binding.context,
        "successor-runtime",
    )
    .unwrap();
    let event = SemanticEvent::for_context(
        &binding.context,
        "successor-runtime",
        "child-routine-terminal",
        1,
        1,
        "check.routine.terminal",
        "pass",
    )
    .unwrap();
    assert!(store.append(&event).unwrap());

    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "observe", "query", "--target", "child"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["context_id"], binding.context.context_id());
    assert_eq!(value["event_count"], 1);
    assert_eq!(value["events"][0]["event_id"], "child-routine-terminal");
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
