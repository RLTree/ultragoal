use super::{CollectionEntryKind, collection_entries};
use crate::context::{BuildRequest, LiveContext};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(_label: &str) -> Self {
        let root = PathBuf::from("/tmp").join(format!(
            "ugw-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "walk@example.invalid"]);
        git(&root, &["config", "user.name", "Typed Walk Test"]);
        fs::write(root.join("tracked.txt"), b"tracked\n").unwrap();
        git(&root, &["add", "tracked.txt"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        Self { root }
    }

    fn session(&self) -> crate::context::ReadSession {
        LiveContext::build(BuildRequest::new(&self.root))
            .unwrap()
            .begin_read_session()
            .unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

#[cfg(unix)]
#[test]
fn typed_collection_preserves_regular_directory_link_and_special_entries() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    let repo = Repo::new("kinds");
    let agents = repo.root.join(".codex/agents");
    fs::create_dir_all(agents.join("empty")).unwrap();
    fs::create_dir_all(agents.join("nested")).unwrap();
    fs::write(agents.join("single.toml"), b"single\n").unwrap();
    fs::write(agents.join("nested/inside.toml"), b"nested\n").unwrap();
    let hard_source = repo.root.join("hard-source.toml");
    fs::write(&hard_source, b"hard\n").unwrap();
    fs::hard_link(&hard_source, agents.join("hard.toml")).unwrap();
    symlink("single.toml", agents.join("link.toml")).unwrap();
    let fifo = agents.join("fifo");
    let fifo_name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) }, 0);
    let _listener = UnixListener::bind(agents.join("socket")).unwrap();

    let reads = repo.session();
    let actual = collection_entries(&reads, &agents)
        .unwrap()
        .into_iter()
        .map(|entry| {
            (
                entry
                    .path
                    .strip_prefix(&agents)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                entry.kind,
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([
            ("empty".to_owned(), CollectionEntryKind::Directory),
            ("fifo".to_owned(), CollectionEntryKind::Special),
            (
                "hard.toml".to_owned(),
                CollectionEntryKind::Regular { single_link: false },
            ),
            ("link.toml".to_owned(), CollectionEntryKind::Symlink),
            ("nested".to_owned(), CollectionEntryKind::Directory),
            (
                "nested/inside.toml".to_owned(),
                CollectionEntryKind::Regular { single_link: true },
            ),
            (
                "single.toml".to_owned(),
                CollectionEntryKind::Regular { single_link: true },
            ),
            ("socket".to_owned(), CollectionEntryKind::Special),
        ])
    );
    reads.revalidate().unwrap();
}

#[test]
fn typed_collection_pins_directories_for_final_revalidation() {
    let repo = Repo::new("revalidation");
    let agents = repo.root.join(".codex/agents");
    fs::create_dir_all(&agents).unwrap();
    fs::write(agents.join("one.toml"), b"one\n").unwrap();
    let reads = repo.session();
    let entries = collection_entries(&reads, &agents).unwrap();
    assert_eq!(entries.len(), 1);
    fs::create_dir(agents.join("late-empty-directory")).unwrap();
    assert!(reads.revalidate().is_err());
}
