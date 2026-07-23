use super::*;
use std::process::Command;

#[test]
fn public_diagnose_projects_routine_state_when_inventory_is_incomplete() {
    let repo = Repository::new("routine-diagnosis-partial-fit");
    install_routine(&repo.root, 1);
    fs::remove_file(repo.root.join("migration/authority-routes.json")).unwrap();
    let home = empty_home("routine-diagnosis-partial-fit");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "diagnose"]).unwrap() else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 1);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["schema_version"], "RoutineDiagnosis-v1");
    assert_eq!(value["status"], "no_record");
    assert_eq!(value["effect_state"], "no_effect");
    assert_eq!(value["claim_effect"], "none");
    assert!(
        value["repository_root_id"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(!String::from_utf8_lossy(&streams.stdout).contains(repo.root.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn public_diagnose_retains_inventory_failure_when_no_routine_surface_exists() {
    let repo = Repository::new("routine-diagnosis-no-surface");
    fs::remove_file(repo.root.join("migration/authority-routes.json")).unwrap();
    let home = empty_home("routine-diagnosis-no-surface");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "diagnose"]).unwrap() else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let output = String::from_utf8(streams.stderr).unwrap();
    assert!(output.contains("successor_runtime_inventory_unavailable"));
    assert!(!output.contains(repo.root.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn explicit_target_diagnosis_uses_the_confined_child_binding() {
    let repo = Repository::new("routine-diagnosis-child");
    let child = repo.add_nested_repository("child");
    install_routine(&child, 2);
    let home = empty_home("routine-diagnosis-child");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "diagnose", "--target", "child"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 1);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    let child_context = super::super::routine::current_diagnosis_binding(
        &repo.root,
        Some("child"),
        &read_context(&repo.root).unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(value["context_id"], child_context.context().context_id());
    assert_eq!(
        value["candidate_id"],
        child_context.plan().binding().candidate_id()
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(home).unwrap();
}

#[cfg(unix)]
#[test]
fn explicit_target_diagnosis_rejects_a_substituted_directory_without_writes_or_path_echo() {
    let repo = Repository::new("routine-diagnosis-substituted-target");
    let child = repo.add_nested_repository("child");
    let substituted = repo.root.join("substituted");
    std::os::unix::fs::symlink(&child, &substituted).unwrap();
    let home = empty_home("routine-diagnosis-substituted-target");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "diagnose", "--target", "substituted"]).unwrap()
    else {
        panic!("expected invocation")
    };
    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    let output = String::from_utf8_lossy(&streams.stdout);
    assert!(!output.contains(repo.root.to_str().unwrap()));
    assert!(!output.contains(child.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    fs::remove_dir_all(home).unwrap();
}

fn install_routine(root: &Path, value: u8) {
    fs::create_dir_all(root.join("config")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("config/routine-public.json"),
        include_bytes!("../../../../../templates/config/routine-public.json"),
    )
    .unwrap();
    fs::write(
        root.join("config/routines.json"),
        include_bytes!("../../../../../templates/config/routines.json"),
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        format!("pub fn value() -> u8 {{ {value} }}\n"),
    )
    .unwrap();
    git(root, &["add", "config", "src/lib.rs"]);
    git(root, &["commit", "-qm", "add routine configuration"]);
}

fn empty_home(label: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("{label}-{}-home", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::canonicalize(home).unwrap()
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
