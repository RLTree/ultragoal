use super::*;

pub(crate) fn canonical_arguments(node: &NodeSpec) -> Vec<String> {
    let delay = if node.delay_seconds == 0 {
        String::new()
    } else {
        format!(
            "HUL_ROUTINE_END=$((SECONDS + {})); while [ \"$SECONDS\" -lt \"$HUL_ROUTINE_END\" ]; do :; done; ",
            node.delay_seconds
        )
    };
    let settle = "exec 1>&- 2>&-; HUL_ROUTINE_SETTLE=0; while [ \"$HUL_ROUTINE_SETTLE\" -lt 4096 ]; do HUL_ROUTINE_SETTLE=$((HUL_ROUTINE_SETTLE + 1)); done";
    let report = match node.action {
        "pass" => format!(
            "printf '%s' \"$HUL_ROUTINE_NODE_ID\" > 'target/routine/{}/result.txt'; printf '{{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\"; {settle}",
            node.id,
        ),
        "fail" => format!(
            "printf '{{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"failed\",\"behavior_observed\":true}}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\"; {settle}; exit 7"
        ),
        _ => panic!("unknown action"),
    };
    vec![
        "-c".to_owned(),
        format!(
            "exec 2> 'target/routine/{}/debug.txt'; {delay}{report}",
            node.id
        ),
    ]
}

pub(crate) fn provision_host_state(home: &Path) {
    let components = [
        ".codex",
        ".codex/state",
        ".codex/state/harness-ultragoal",
        ".codex/state/harness-ultragoal/routine-public",
        ".codex/state/harness-ultragoal/routine-public/authority",
        ".codex/state/harness-ultragoal/routine-public/adapter",
    ];
    for component in components {
        let path = home.join(component);
        fs::create_dir_all(&path).unwrap();
        set_mode(&path, 0o700);
    }
    let lock = home.join(".codex/state/harness-ultragoal/routine-public/adapter/adapter.lock");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&lock)
        .unwrap();
    file.write_all(b"routine-public-lock-v1\n").unwrap();
    file.sync_all().unwrap();
}

pub(crate) fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

pub(crate) fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

pub(crate) fn git_output(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output.stdout
}

pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", raw_sha(bytes))
}

pub(crate) fn raw_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
