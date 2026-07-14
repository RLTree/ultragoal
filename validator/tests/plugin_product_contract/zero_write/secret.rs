const SECRET: &str = "TOP_SECRET_DEPENDENCY_CLOSURE_018";
const PRIVATE_ARGUMENTS: &[&str] = &[
    "private-target-closure-018",
    "private-finding-closure-018",
    "private-spec-closure-018.json",
    "private-registry-closure-018.json",
];

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    id: String,
    args: Vec<String>,
    source_marker: String,
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: &'static str,
    byte_length: u64,
    content_sha256: String,
    unix_mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    tree: Vec<SnapshotRow>,
    status: Vec<u8>,
}

struct Repository {
    root: PathBuf,
}

impl Repository {
    #[cfg(unix)]
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-plugin-product-zero-write-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "zero-write@example.invalid"],
        );
        git(&root, &["config", "user.name", "Zero Write Probe"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            b"{\"name\":\"harness-ultragoal\",\"version\":\"0.0.0-probe\"}\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("nested/empty")).unwrap();
        fs::write(root.join("nested/tracked.txt"), b"tracked baseline\n").unwrap();
        fs::set_permissions(
            root.join("nested/tracked.txt"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        std::os::unix::fs::symlink("tracked.txt", root.join("nested/tracked-link")).unwrap();
        fs::write(root.join(".gitignore"), b"validation_artifacts/\n").unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "zero-write fixture"]);
        fs::write(
            root.join("nested/tracked.txt"),
            b"tracked dirty state retained\n",
        )
        .unwrap();
        fs::write(root.join("private-canary.txt"), format!("{SECRET}\n")).unwrap();
        Self { root }
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn operations() -> Vec<Operation> {
    let value: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json")).unwrap();
    serde_json::from_value(value["candidate_closure"]["executable_read_operations"].clone())
        .unwrap()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(unix)]
fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
    } else {
        Vec::new()
    }
}

#[cfg(unix)]
fn push_row(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
    let metadata = fs::symlink_metadata(path).unwrap();
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
        relative_path: path.strip_prefix(root).unwrap().to_path_buf(),
        kind,
        byte_length: metadata.len(),
        content_sha256: digest(&content(path, &metadata)),
        unix_mode: metadata.mode(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            push_row(root, &entry, rows);
        }
    }
}

#[cfg(unix)]
fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    push_row(root, root, &mut rows);
    rows
}

fn git(root: &Path, args: &[&str]) -> Output {
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
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

#[cfg(unix)]
fn observe(root: &Path) -> Observation {
    Observation {
        tree: snapshot(root),
        status: git(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )
        .stdout,
    }
}

fn execute(current_dir: &Path, args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ultragoal"))
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(current_dir)
        .output()
        .unwrap()
}
