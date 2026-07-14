use super::*;

pub(crate) struct Fixture {
    pub(crate) container: PathBuf,
    pub(crate) root: PathBuf,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let short = &label[..label.len().min(4)];
        let container = PathBuf::from("/tmp").join(format!(
            "huf-{short}-{}-{}",
            std::process::id(),
            nonce % 1_000_000_000
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).unwrap();
        Self { container, root }
    }

    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(crate) fn init_git(&self) {
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&self.root)
                .status()
                .unwrap()
                .success()
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

#[test]
pub(crate) fn anchored_local_inspect_plan_verify_are_recursive_tree_zero_write() {
    let fixture = Fixture::new("zero-write");
    fixture.write(".codex/AGENTS.md", b"canonical\n");
    fixture.write("unrelated-user.txt", b"preserve\n");
    fixture.init_git();
    let status_before = git_status(&fixture.root);
    let before = snapshot(&fixture.root);
    let mut reader = LocalRepository::open(&fixture.root).unwrap();
    let first_binding = reader.root_binding().unwrap();
    assert_eq!(reader.root_binding().unwrap(), first_binding);
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse(".codex/AGENTS.md").unwrap(), 1024)
            .unwrap(),
        Some(b"canonical\n".to_vec())
    );
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse("missing/FILE.md").unwrap(), 1024)
            .unwrap(),
        None
    );
    let desired = desired(vec![file(
        ".codex/AGENTS.md",
        b"canonical\n",
        Ownership::HarnessGenerated,
        &[],
    )]);
    let inspection = inspect(FitMode::Retrofit, &desired, &mut reader).unwrap();
    assert_eq!(inspection.classification(), &RepositoryClass::AlreadyFitted);
    assert!(plan(&inspection, &desired).unwrap().mutations().is_empty());
    assert!(verify(&desired, &mut reader).unwrap().idempotent());
    assert_eq!(git_status(&fixture.root), status_before);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn local_reader_rejects_exact_plus_ascii_casefold_aliases_at_every_component() {
    let leaf = Fixture::new("leaf-case-alias");
    leaf.write("AGENTS.md", b"exact");
    if create_distinct_file(&leaf.root.join("Agents.md"), b"alias") {
        rejected(&leaf, "AGENTS.md");
    }

    let ancestor = Fixture::new("ancestor-case-alias");
    ancestor.write(".codex/AGENTS.md", b"exact");
    if create_distinct_directory(&ancestor.root.join(".CODEX")) {
        fs::write(ancestor.root.join(".CODEX/AGENTS.md"), b"alias").unwrap();
        rejected(&ancestor, ".codex/AGENTS.md");
    }
}

#[test]
pub(crate) fn local_reader_distinguishes_alias_only_exact_only_and_safe_siblings() {
    let leaf_alias = Fixture::new("leaf-alias-only");
    leaf_alias.write("Agents.md", b"alias");
    rejected(&leaf_alias, "AGENTS.md");

    let ancestor_alias = Fixture::new("ancestor-alias-only");
    ancestor_alias.write(".CODEX/AGENTS.md", b"alias");
    rejected(&ancestor_alias, ".codex/AGENTS.md");

    let exact = Fixture::new("exact-and-sibling");
    exact.write(".codex/AGENTS.md", b"exact");
    exact.write(".codex/AGENT.md", b"safe sibling");
    let before = snapshot(&exact.root);
    let mut reader = LocalRepository::open(&exact.root).unwrap();
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse(".codex/AGENTS.md").unwrap(), 1024)
            .unwrap(),
        Some(b"exact".to_vec())
    );
    assert_eq!(snapshot(&exact.root), before);
}

#[test]
pub(crate) fn local_reader_rejects_links_special_objects_and_traversal() {
    for raw in ["../AGENTS.md", "a/../../AGENTS.md", "./AGENTS.md"] {
        assert_eq!(
            CanonicalPath::parse(raw).unwrap_err().id(),
            FitErrorId::InvalidPath
        );
    }

    let leaf_link = Fixture::new("leaf-link");
    leaf_link.write("real", b"real");
    symlink("real", leaf_link.root.join("AGENTS.md")).unwrap();
    rejected(&leaf_link, "AGENTS.md");

    let ancestor_link = Fixture::new("ancestor-link");
    fs::create_dir(ancestor_link.root.join("real")).unwrap();
    fs::write(ancestor_link.root.join("real/AGENTS.md"), b"real").unwrap();
    symlink("real", ancestor_link.root.join(".codex")).unwrap();
    rejected(&ancestor_link, ".codex/AGENTS.md");

    let hardlink = Fixture::new("hardlink");
    hardlink.write("real", b"real");
    fs::hard_link(hardlink.root.join("real"), hardlink.root.join("AGENTS.md")).unwrap();
    rejected(&hardlink, "AGENTS.md");

    let fifo = Fixture::new("fifo");
    let fifo_path = fifo.root.join("AGENTS.md");
    let raw = CString::new(fifo_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(raw.as_ptr(), 0o600) }, 0);
    rejected(&fifo, "AGENTS.md");

    let socket = Fixture::new("socket");
    let _listener = std::os::unix::net::UnixListener::bind(socket.root.join("AGENTS.md")).unwrap();
    rejected(&socket, "AGENTS.md");
}

#[test]
pub(crate) fn root_replacement_invalidates_open_reader_without_following_new_tree() {
    let fixture = Fixture::new("root-replacement");
    fixture.write("AGENTS.md", b"original");
    let mut reader = LocalRepository::open(&fixture.root).unwrap();
    let moved = fixture.container.join("moved");
    fs::rename(&fixture.root, &moved).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    fs::write(fixture.root.join("AGENTS.md"), b"substitute").unwrap();
    assert_eq!(
        reader.root_binding().unwrap_err().id(),
        FitErrorId::StaleBinding
    );
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse("AGENTS.md").unwrap(), 1024)
            .unwrap_err()
            .id(),
        FitErrorId::StaleBinding
    );
}

#[test]
pub(crate) fn local_reader_enforces_requested_and_global_resource_limits() {
    let fixture = Fixture::new("limits");
    fixture.write("AGENTS.md", b"12345");
    let mut reader = LocalRepository::open(&fixture.root).unwrap();
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse("AGENTS.md").unwrap(), 4)
            .unwrap_err()
            .id(),
        FitErrorId::ResourceLimit
    );
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse("AGENTS.md").unwrap(), usize::MAX)
            .unwrap_err()
            .id(),
        FitErrorId::ResourceLimit
    );
}

pub(crate) fn rejected(fixture: &Fixture, relative: &str) {
    let before = snapshot(&fixture.root);
    let mut reader = LocalRepository::open(&fixture.root).unwrap();
    assert_eq!(
        reader
            .read_file(&CanonicalPath::parse(relative).unwrap(), 1024)
            .unwrap_err()
            .id(),
        FitErrorId::UnsafeObject
    );
    assert_eq!(snapshot(&fixture.root), before);
}

pub(crate) fn create_distinct_file(path: &Path, bytes: &[u8]) -> bool {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes).unwrap();
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => panic!("failed to create case-fold fixture: {error}"),
    }
}

pub(crate) fn create_distinct_directory(path: &Path) -> bool {
    match fs::create_dir(path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => panic!("failed to create case-fold fixture: {error}"),
    }
}

pub(crate) fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    output.stdout
}
