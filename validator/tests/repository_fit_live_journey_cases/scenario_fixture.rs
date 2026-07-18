use super::*;

pub(crate) const BASE: &str = "/tmp/hul-repository-fit-live-journeys-067";
pub(crate) const PRIVATE_CANARY: &str = "REPOSITORY_FIT_PRIVATE_COMMAND_CANARY_067";
pub(crate) static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Case {
    pub(crate) id: String,
    pub(crate) class: String,
    pub(crate) expect: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadOperation {
    pub(crate) id: String,
    pub(crate) args: Vec<String>,
    pub(crate) schema_version: String,
    pub(crate) exit_code: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Catalog {
    pub(crate) schema_version: String,
    pub(crate) temporary_root: String,
    pub(crate) supported_host: String,
    pub(crate) claim_effect: String,
    pub(crate) production_permit_issuer: String,
    pub(crate) public_apply_dispatch: String,
    pub(crate) cases: Vec<Case>,
    pub(crate) read_operations: Vec<ReadOperation>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SnapshotRow {
    pub(crate) relative_path: PathBuf,
    pub(crate) kind: &'static str,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) unix_mode: u32,
    pub(crate) byte_length: u64,
    pub(crate) content_sha256: String,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Observation {
    pub(crate) tree: Vec<SnapshotRow>,
    pub(crate) git_status: Vec<u8>,
}

pub(crate) struct CommandRepository {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl CommandRepository {
    pub(crate) fn new() -> Self {
        let container = PathBuf::from(BASE).join(format!(
            "command-zero-write-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = container.join("repo");
        fs::create_dir_all(root.join("nested/empty")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "--quiet"]);
        git(
            &root,
            &["config", "user.email", "fit-command@example.invalid"],
        );
        git(&root, &["config", "user.name", "Repository Fit Command"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        git(&root, &["config", "gc.auto", "0"]);
        git(&root, &["config", "maintenance.auto", "false"]);
        fs::write(root.join("tracked.txt"), b"tracked baseline\n").unwrap();
        fs::write(root.join("nested/linked.txt"), b"linked bytes\n").unwrap();
        std::os::unix::fs::symlink("linked.txt", root.join("nested/linked-symlink")).unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "--quiet", "-m", "command baseline"]);
        fs::write(root.join("tracked.txt"), b"tracked dirty user edit\n").unwrap();
        fs::write(root.join("private-canary.txt"), PRIVATE_CANARY).unwrap();
        Self { container, root }
    }

    pub(crate) fn run(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ultragoal"))
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .arg("--root")
            .arg(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for CommandRepository {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn source(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative)).unwrap()
}

pub(crate) fn rust_tree(relative: &str) -> String {
    fn collect(path: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    collect(&repository_root().join(relative), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn catalog() -> Catalog {
    serde_json::from_str(&source("fixtures/repository-fit-live-journeys/cases.json")).unwrap()
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
    } else {
        Vec::new()
    }
}

pub(crate) fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
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
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        unix_mode: metadata.mode(),
        byte_length: metadata.len(),
        content_sha256: digest(&content(path, &metadata)),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            visit(root, &entry, rows);
        }
    }
}

pub(crate) fn tree(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Output {
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

pub(crate) fn observe(root: &Path) -> Observation {
    let git_status = git(
        root,
        &[
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )
    .stdout;
    Observation {
        tree: tree(root),
        git_status,
    }
}
