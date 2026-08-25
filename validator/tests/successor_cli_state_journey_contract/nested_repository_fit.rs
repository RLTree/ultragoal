use super::PRIVATE_CANARY;
use super::assertions::{assert_diagnostic, assert_payload_any_exit};
use super::snapshot::observe;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct NestedFitRepository {
    container: PathBuf,
    root: PathBuf,
}

impl NestedFitRepository {
    fn new() -> Self {
        let container = std::env::temp_dir().join(format!(
            "hul-successor-nested-fit-default-root-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repository");
        fs::create_dir_all(&root).expect("create nested repository");
        let container = fs::canonicalize(container).expect("canonical fixture container");
        let root = fs::canonicalize(root).expect("canonical nested repository");

        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "nested-fit@example.invalid"],
        );
        git(&root, &["config", "user.name", "Nested Fit Journey"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        fs::write(root.join("tracked.txt"), b"tracked baseline\n").expect("write tracked fixture");
        git(&root, &["add", "tracked.txt"]);
        git(&root, &["commit", "-qm", "nested fit fixture"]);
        fs::write(root.join("tracked.txt"), b"dirty tracked bytes retained\n")
            .expect("dirty tracked fixture");
        fs::write(root.join("private-canary.txt"), PRIVATE_CANARY).expect("write private canary");

        Self { container, root }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ultragoal"))
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.container)
            .args(args)
            .output()
            .expect("fresh successor binary executes")
    }
}

impl Drop for NestedFitRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

#[test]
fn default_root_plans_a_nested_relative_target_without_writes() {
    let repository = NestedFitRepository::new();
    let before = observe(&repository.root);

    let inspect = repository.run(&["--json", "fit", "inspect", "--target", "repository"]);
    assert_payload_any_exit(&inspect, &[0, 1], "RepositoryFitInspect-v1", PRIVATE_CANARY);
    assert_eq!(observe(&repository.root), before, "fit inspect wrote");

    let plan_one = repository.run(&["--json", "fit", "plan", "--target", "repository"]);
    let plan_one_value =
        assert_payload_any_exit(&plan_one, &[0, 1], "RepositoryFitPlan-v1", PRIVATE_CANARY);
    assert_eq!(plan_one_value["effect"], "read");
    assert_eq!(plan_one_value["claim_effect"], "none");
    assert!(
        plan_one_value["plan"]["mutations"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "fresh nested target should have a non-empty fit plan"
    );
    assert_eq!(observe(&repository.root), before, "fit plan wrote");

    let plan_two = repository.run(&["--json", "fit", "plan", "--target", "repository"]);
    assert_payload_any_exit(&plan_two, &[0, 1], "RepositoryFitPlan-v1", PRIVATE_CANARY);
    assert_eq!(plan_two.stdout, plan_one.stdout, "repeat fit plan drifted");
    assert_eq!(observe(&repository.root), before, "repeat fit plan wrote");

    let verify = repository.run(&["--json", "fit", "verify", "--target", "repository"]);
    assert_payload_any_exit(
        &verify,
        &[0, 1],
        "RepositoryFitVerification-v1",
        PRIVATE_CANARY,
    );
    assert_eq!(observe(&repository.root), before, "fit verify wrote");

    for invalid_target in [repository.root.to_str().unwrap(), "../repository"] {
        let invalid = repository.run(&["--json", "fit", "plan", "--target", invalid_target]);
        let value = assert_diagnostic(
            &invalid,
            2,
            "harness-ultragoal.cli-error.v1",
            PRIVATE_CANARY,
        );
        assert_eq!(value["error_id"], "CLI_INVALID_CONFINED_PATH");
        assert_eq!(observe(&repository.root), before, "invalid target wrote");
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("/usr/bin/git")
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(root)
        .output()
        .expect("git fixture command");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}
