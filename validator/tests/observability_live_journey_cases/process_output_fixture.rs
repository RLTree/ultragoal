use super::*;

pub(crate) fn output_with_timeout(mut command: Command, timeout: Duration) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn source-built ultragoal");
    let mut stdout = child.stdout.take().expect("captured stdout");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).expect("read stdout");
        bytes
    });
    let mut stderr = child.stderr.take().expect("captured stderr");
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).expect("read stderr");
        bytes
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("poll source-built ultragoal") {
            let stdout = stdout_reader.join().expect("join stdout reader");
            let stderr = stderr_reader.join().expect("join stderr reader");
            return Output {
                status,
                stdout,
                stderr,
            };
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            panic!("source-built ultragoal exceeded {timeout:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("/usr/bin/git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
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
        .expect("git status observation");
    assert!(output.status.success(), "git status: {output:?}");
    output.stdout
}

pub(crate) fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).expect("read snapshot file")
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path)
            .expect("read snapshot symlink")
            .as_os_str()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    }
}

pub(crate) fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
    let metadata = fs::symlink_metadata(path).expect("snapshot metadata");
    let kind = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else if metadata.file_type().is_symlink() {
        "symlink"
    } else {
        "special"
    };
    rows.push(SnapshotRow {
        relative_path: path
            .strip_prefix(root)
            .expect("snapshot prefix")
            .to_path_buf(),
        kind,
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        unix_mode: metadata.mode(),
        byte_length: metadata.len(),
        content_sha256: format!("sha256:{:x}", Sha256::digest(content(path, &metadata))),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            visit(root, &entry, rows);
        }
    }
}

pub(crate) fn observe(root: &Path) -> Observation {
    let git_status = git_status(root);
    let mut tree = Vec::new();
    visit(root, root, &mut tree);
    Observation { tree, git_status }
}

pub(crate) fn machine(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("machine JSON")
}

pub(crate) fn assert_private_absent(output: &Output, root: &Path) {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    let text = String::from_utf8_lossy(&bytes);
    let root_text = root.to_string_lossy();
    for private in [
        PRIVATE_TOKEN,
        PRIVATE_PATH,
        PRIVATE_EMAIL,
        root_text.as_ref(),
    ] {
        assert!(!text.contains(private), "private output: {text}");
    }
}

pub(crate) fn assert_payload_repeat_zero_write(
    repository: &JourneyRepository,
    args: &[&str],
    expected_exits: &[i32],
    schema: &str,
) -> Value {
    let before = observe(repository.root());
    let first = repository.run(args);
    assert!(
        expected_exits.contains(&first.status.code().unwrap_or(-1)),
        "{args:?}: {first:?}"
    );
    assert!(first.stderr.is_empty(), "{args:?}: {first:?}");
    assert_private_absent(&first, repository.root());
    let value = machine(&first.stdout);
    assert_eq!(value["schema_version"], schema, "{args:?}");
    assert_eq!(observe(repository.root()), before, "hidden write: {args:?}");

    let second = repository.run(args);
    assert_eq!(second.status.code(), first.status.code(), "{args:?}");
    assert_eq!(
        second.stdout, first.stdout,
        "nondeterministic stdout: {args:?}"
    );
    assert_eq!(
        second.stderr, first.stderr,
        "nondeterministic stderr: {args:?}"
    );
    assert_eq!(
        observe(repository.root()),
        before,
        "repeat hidden write: {args:?}"
    );
    value
}

pub(crate) fn assert_diagnostic_zero_write(
    repository: &JourneyRepository,
    args: &[&str],
    expected_exit: i32,
    diagnostic_id: &str,
) -> Value {
    let before = observe(repository.root());
    let output = repository.run(args);
    assert_eq!(output.status.code(), Some(expected_exit), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_private_absent(&output, repository.root());
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], diagnostic_id);
    assert_eq!(
        observe(repository.root()),
        before,
        "hidden diagnostic write"
    );
    value
}

pub(crate) fn public_binding(repository: &JourneyRepository) -> Binding {
    let value = assert_payload_repeat_zero_write(
        repository,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(value["store_status"], "absent");
    Binding {
        context_id: value["context_id"]
            .as_str()
            .expect("query context id")
            .to_owned(),
        candidate_id: value["candidate_id"]
            .as_str()
            .expect("query candidate id")
            .to_owned(),
        source_id: value["source_id"]
            .as_str()
            .expect("query source id")
            .to_owned(),
    }
}
