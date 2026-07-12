use super::fixtures::{command, live_loop_timing_receipt_arg};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::Instant;

const TYPED_LINE_CAPS_REFUSAL: &str = r#"{"schema_version":"harness-ultragoal.cli-error.v1","error_id":"CLI_UNKNOWN_SUBCOMMAND","exit_code":2}"#;

#[test]
fn line_caps_summary_is_not_replay_authority_and_public_refusal_is_stable() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-line-caps-replay",
    );
    fs::create_dir_all(root.join("validator/src")).expect("source dir");
    git(&root, &["init", "-q"]);
    git(&root, &["config", "gc.auto", "0"]);
    git(&root, &["config", "maintenance.auto", "false"]);
    git(&root, &["config", "maintenance.autoDetach", "false"]);
    fs::write(root.join("validator/src/lib.rs"), "pub fn ok() {}\n").expect("source");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["validator/src/lib.rs"]}),
    )
    .expect("manifest");
    let line_caps = crate::cli::line_caps::LineCapsCommand {
        receipt: PathBuf::from("validation_artifacts/observability/line-cap-check.json"),
        jobs: Some(8),
    };
    assert_eq!(
        crate::cli::line_caps::run(&root, &line_caps).expect("line caps command"),
        0
    );

    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let command = command(Some("line_caps_check"), live_loop_timing_receipt_arg());
    let surface = surface_by_id("line_caps_check").expect("line caps surface");
    let inputs = ChangedInputs::collect(&root, &candidate, &command.tier, &command.cache_mode);
    let input_digest = super::super::super::super::graph::surface_input_digest(
        surface,
        &candidate,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    let cache_key = super::super::super::super::graph::verified_local_cache_key(
        surface,
        &input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let before = repository_snapshot(&root);

    for _ in 0..2 {
        let replay = super::super::cache_replay::verified_local_hit(
            &root,
            surface,
            &candidate,
            &input_digest,
            &command,
            &cache_key,
            Instant::now(),
            super::super::ObservationMode::FullRoundtrip,
        );
        assert!(
            replay.is_none(),
            "a line-cap summary must not substitute for exact command-observation authority"
        );
    }

    let first_refusal = typed_line_caps_refusal();
    let second_refusal = typed_line_caps_refusal();
    assert_eq!(first_refusal, second_refusal);
    assert_eq!(first_refusal, TYPED_LINE_CAPS_REFUSAL);
    assert!(!first_refusal.contains(root.to_string_lossy().as_ref()));
    assert!(!first_refusal.contains("validator/src/lib.rs"));

    let after = repository_snapshot(&root);
    assert_eq!(after, before, "read-only refusal paths changed the fixture");

    fs::remove_dir_all(root).expect("cleanup line caps replay");
}

fn typed_line_caps_refusal() -> String {
    let args = ["--json", "line-caps", "check", "--strict", "--jobs", "8"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    std::panic::catch_unwind(|| crate::argument_parser::parse_public_args_from(args))
        .expect("public parser must not panic for a retired legacy route")
        .expect_err("retired legacy route must remain unavailable")
}

#[derive(Debug, Eq, PartialEq)]
struct RepositorySnapshot {
    tree: Vec<(String, Vec<u8>)>,
    status: Vec<u8>,
}

fn repository_snapshot(root: &Path) -> RepositorySnapshot {
    RepositorySnapshot {
        status: git_output(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        ),
        tree: recursive_tree(root),
    }
}

fn recursive_tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, current: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot relative path")
                .to_string_lossy();
            let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
            let permissions = permission_marker(&metadata);
            if metadata.is_dir() {
                rows.push((format!("directory:{permissions}:{relative}"), Vec::new()));
                collect(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    format!("symlink:{permissions}:{relative}"),
                    fs::read_link(&path)
                        .expect("snapshot symlink")
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else if metadata.is_file() {
                rows.push((
                    format!("file:{permissions}:{relative}"),
                    fs::read(&path).expect("snapshot file"),
                ));
            } else {
                rows.push((format!("special:{permissions}:{relative}"), Vec::new()));
            }
        }
    }

    let mut rows = Vec::new();
    collect(root, root, &mut rows);
    rows
}

#[cfg(unix)]
fn permission_marker(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permission_marker(metadata: &fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
}

fn git(root: &Path, args: &[&str]) {
    let output = git_command(root, args).output().expect("git command");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

fn git_output(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = git_command(root, args).output().expect("git command");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output.stdout
}

fn git_command(root: &Path, args: &[&str]) -> ProcessCommand {
    let mut command = ProcessCommand::new("/usr/bin/git");
    command
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");
    command
}
