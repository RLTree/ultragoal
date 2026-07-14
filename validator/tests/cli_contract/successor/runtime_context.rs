use super::successor::runtime::{RuntimeErrorId, inspect_context};
use super::successor::{
    EffectClass, InspectTarget, OptionArgument, OptionName, OutputMode, ParseOutcome, ParsedValue,
    SuccessorCommand, parse_args,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REPOSITORY: AtomicU64 = AtomicU64::new(0);

#[path = "state_views.rs"]
mod state_views;

struct Repository {
    root: PathBuf,
}

impl Repository {
    fn new(label: &str) -> Self {
        let sequence = NEXT_REPOSITORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-successor-runtime-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create runtime repository");
        let repository = Self { root };
        repository.git(&["init", "-q"]);
        repository.git(&["config", "user.email", "runtime@example.invalid"]);
        repository.git(&["config", "user.name", "Runtime Test"]);
        fs::write(repository.root.join("tracked.txt"), b"tracked\n")
            .expect("write tracked fixture");
        repository.git(&["add", "tracked.txt"]);
        repository.git(&["commit", "-q", "-m", "fixture"]);
        repository
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .status()
            .expect("run Git fixture command");
        assert!(status.success(), "git {args:?}");
    }

    fn status(&self) -> Vec<u8> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .expect("read Git status");
        assert!(output.status.success(), "git status failed");
        output.stdout
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn parsed(args: &[&str]) -> super::successor::ParsedInvocation {
    let ParseOutcome::Invocation(invocation) =
        parse_args(args.iter().copied()).expect("parse successor invocation")
    else {
        panic!("expected invocation");
    };
    invocation
}

fn tree_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, path: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .expect("read snapshot directory")
            .map(|entry| entry.expect("read snapshot entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot path under root")
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).expect("read snapshot metadata");
            if metadata.is_dir() {
                rows.push((format!("directory:{relative}"), Vec::new()));
                collect(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    format!("symlink:{relative}"),
                    fs::read_link(&path)
                        .expect("read snapshot symlink")
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((
                    format!("file:{relative}"),
                    fs::read(&path).expect("read snapshot file"),
                ));
            }
        }
    }

    let mut rows = Vec::new();
    collect(root, root, &mut rows);
    rows
}

#[test]
fn inspect_context_returns_a_current_dirty_read_context_without_writes() {
    let repository = Repository::new("accepted-dirty");
    fs::write(repository.root.join("dirty-canary.txt"), b"dirty\n").expect("write dirty fixture");
    let invocation = parsed(&["--json", "inspect", "context"]);
    let before_tree = tree_snapshot(&repository.root);
    let before_status = repository.status();

    let context = inspect_context(&repository.root, &invocation).expect("inspect live context");

    assert!(context.candidate().dirty);
    assert_eq!(context.effect().selected, EffectClass::Read);
    assert_eq!(
        context.worktree_root(),
        repository
            .root
            .canonicalize()
            .expect("canonical fixture repository")
    );
    context
        .revalidate()
        .expect("returned context remains current");
    let json = String::from_utf8(context.to_canonical_json().expect("serialize context"))
        .expect("context JSON is UTF-8");
    assert!(json.contains("\"schema_version\":\"LiveContext-v1\""));
    assert_eq!(tree_snapshot(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
fn runtime_rejects_every_non_context_command_before_repository_access() {
    let canary = Path::new("/missing/runtime-command-canary-7183");
    let error =
        inspect_context(canary, &parsed(&["next"])).expect_err("wrong successor command must fail");
    assert_eq!(error.id(), RuntimeErrorId::WrongCommand);
    assert_eq!(error.stable_id(), "successor_runtime_wrong_command");
    assert!(!error.to_string().contains("runtime-command-canary-7183"));
}

#[test]
fn runtime_rejects_effect_drift_and_arguments_before_repository_access() {
    let canary = Path::new("/missing/runtime-effect-canary-6421");
    let mut effect = parsed(&["inspect", "context"]);
    effect.effect = EffectClass::WorkspaceWrite;
    assert_eq!(
        inspect_context(canary, &effect)
            .expect_err("effect drift must fail")
            .id(),
        RuntimeErrorId::EffectMismatch
    );

    let mut arguments = parsed(&["inspect", "context"]);
    arguments.arguments.push(OptionArgument {
        name: OptionName::ApproveExport,
        value: ParsedValue::Flag,
    });
    assert_eq!(
        inspect_context(canary, &arguments)
            .expect_err("unexpected arguments must fail")
            .id(),
        RuntimeErrorId::UnexpectedArguments
    );
}

#[test]
fn context_failures_have_fixed_bounded_non_echo_errors() {
    let canary = "runtime-context-path-canary-9937";
    let invocation = super::successor::ParsedInvocation {
        command: SuccessorCommand::Inspect(InspectTarget::Context),
        effect: EffectClass::Read,
        output_mode: OutputMode::Human,
        arguments: Vec::new(),
    };
    let error = inspect_context(Path::new(canary), &invocation)
        .expect_err("non-repository context must fail");
    assert_eq!(error.id(), RuntimeErrorId::ContextUnavailable);
    assert_eq!(error.to_string(), "successor runtime context unavailable");
    assert!(error.to_string().len() <= 64);
    assert!(!error.to_string().contains(canary));
}
