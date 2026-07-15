use super::*;

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

pub(crate) fn routine_command(root: &Path, home: &Path, binary: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env_clear()
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", binary.parent().unwrap())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(root)
        .arg("--root")
        .arg(root);
    command
}

pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", raw_sha(bytes))
}

pub(crate) fn raw_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
