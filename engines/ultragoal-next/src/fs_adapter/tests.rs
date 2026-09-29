use super::*;
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TREE: AtomicU64 = AtomicU64::new(0);

struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let ordinal = NEXT_TREE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("ug-fs-{}-{stamp}-{ordinal}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn limits() -> Limits {
    Limits {
        max_entries: 100,
        max_depth: 4,
        max_path_bytes: 1024,
        max_file_bytes: 32_768,
        max_total_bytes: 65_536,
    }
}

#[test]
fn stable_membership_and_selected_bytes() {
    let tree = Tree::new();
    fs::create_dir(tree.at("sub")).unwrap();
    fs::write(tree.at("sub/a"), b"alpha").unwrap();
    fs::write(tree.at("b"), b"beta").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let listing = root.enumerate(&limits());
    assert!(listing.complete, "{:?}", listing.issues);
    assert!(
        listing.members.iter().any(|item| item.path == b"sub/a"
            && item.kind == Kind::File
            && item.identity.size == 5)
    );
    assert!(
        listing
            .members
            .iter()
            .any(|item| item.path == b"sub" && item.kind == Kind::Directory)
    );
    let captures = root.capture_selected(&[b"sub/a".to_vec(), b"b".to_vec()], &limits());
    assert_eq!(captures[0].bytes.as_deref(), Some(b"alpha".as_slice()));
    assert_eq!(captures[1].bytes.as_deref(), Some(b"beta".as_slice()));
    assert!(
        captures
            .iter()
            .all(|item| item.problem.is_none() && item.identity.is_some())
    );
    let membership = root.observe(None, &limits());
    assert!(membership.listing.complete);
    assert!(membership.captures.is_empty());
    let selected = snapshot(
        &fs::canonicalize(&tree.0).unwrap(),
        Some(&[b"b".to_vec()]),
        &limits(),
    )
    .unwrap();
    assert_eq!(
        selected.captures[0].bytes.as_deref(),
        Some(b"beta".as_slice())
    );
}

#[test]
fn enumeration_never_opens_file_content() {
    let tree = Tree::new();
    fs::write(tree.at("locked"), b"private").unwrap();
    fs::set_permissions(tree.at("locked"), fs::Permissions::from_mode(0o0)).unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let listing = root.enumerate(&limits());
    assert!(listing.complete);
    assert_eq!(listing.members[0].path, b"locked");
    assert_eq!(listing.members[0].kind, Kind::File);
    let capture = root.capture_selected(&[b"locked".to_vec()], &limits());
    assert!(matches!(capture[0].problem, Some(Problem::Unavailable(_))));
    fs::set_permissions(tree.at("locked"), fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn symlinks_hardlinks_and_fifo_are_rejected_without_external_read() {
    let tree = Tree::new();
    let outside = Tree::new();
    fs::write(outside.at("secret"), b"outside-data").unwrap();
    symlink(outside.at("secret"), tree.at("link")).unwrap();
    symlink(&outside.0, tree.at("parent")).unwrap();
    fs::hard_link(outside.at("secret"), tree.at("hard")).unwrap();
    let fifo = std::ffi::CString::new(tree.at("pipe").as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let paths = [
        b"link".to_vec(),
        b"parent/secret".to_vec(),
        b"hard".to_vec(),
        b"pipe".to_vec(),
    ];
    let captures = root.capture_selected(&paths, &limits());
    assert!(captures.iter().all(|item| item.bytes.is_none()));
    assert_eq!(captures[0].problem, Some(Problem::Unsupported));
    assert!(matches!(captures[1].problem, Some(Problem::Unavailable(_))));
    assert_eq!(captures[2].problem, Some(Problem::Unsupported));
    assert_eq!(captures[3].problem, Some(Problem::Unsupported));
    assert_eq!(fs::read(outside.at("secret")).unwrap(), b"outside-data");
}

#[test]
fn invalid_paths_and_non_utf8_names_are_explicit() {
    let tree = Tree::new();
    let name = std::ffi::OsString::from_vec(vec![b'x', 0xff]);
    let created = fs::write(tree.0.join(&name), b"raw");
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let listing = root.enumerate(&limits());
    assert!(listing.complete);
    if created.is_ok() {
        assert_eq!(listing.members[0].path, vec![b'x', 0xff]);
        assert_eq!(
            root.capture_selected(&[vec![b'x', 0xff]], &limits())[0]
                .bytes
                .as_deref(),
            Some(b"raw".as_slice())
        );
    } else {
        assert!(created.is_err()); // This host refuses the byte name.
        assert!(matches!(
            root.capture_selected(&[vec![b'x', 0xff]], &limits())[0].problem,
            Some(Problem::Unavailable(_))
        ));
    }
    for path in [
        b"/x".to_vec(),
        b"../x".to_vec(),
        b"a/../x".to_vec(),
        b"a//x".to_vec(),
        b"a/./x".to_vec(),
        b"x/".to_vec(),
        b"x\0y".to_vec(),
        Vec::new(),
    ] {
        assert_eq!(
            root.capture_selected(&[path], &limits())[0].problem,
            Some(Problem::InvalidPath)
        );
    }
}

#[test]
fn caller_exclusion_prunes_large_branch_before_entry_limit() {
    let tree = Tree::new();
    fs::create_dir(tree.at("bulk")).unwrap();
    fs::create_dir(tree.at("src")).unwrap();
    fs::create_dir(tree.at("src/bulk")).unwrap();
    for index in 0..24 {
        fs::write(tree.at(&format!("bulk/{index}")), b"bulk").unwrap();
    }
    fs::write(tree.at("src/file"), b"wanted").unwrap();
    fs::write(tree.at("src/bulk/nested"), b"also-excluded").unwrap();
    let root_path = fs::canonicalize(&tree.0).unwrap();
    let root = Root::open(&root_path).unwrap();
    let mut small = limits();
    small.max_entries = 2;
    assert!(!root.enumerate(&small).complete);
    let excluded = vec![b"bulk".to_vec()];
    let listing = root.enumerate_excluding(&small, &excluded);
    assert!(listing.complete, "{:?}", listing.issues);
    assert_eq!(listing.excluded_basenames, excluded);
    assert_eq!(
        listing
            .members
            .iter()
            .map(|m| m.path.as_slice())
            .collect::<Vec<_>>(),
        vec![b"src".as_slice(), b"src/file".as_slice()]
    );
    let selected = [b"bulk/0".to_vec(), b"src/file".to_vec()];
    let snapshot = root.observe_excluding(Some(&selected), &small, &excluded);
    assert!(snapshot.listing.complete);
    assert_eq!(snapshot.captures[0].problem, Some(Problem::Excluded));
    assert!(snapshot.captures[0].bytes.is_none());
    assert_eq!(
        snapshot.captures[1].bytes.as_deref(),
        Some(b"wanted".as_slice())
    );
    assert_eq!(
        root.capture_selected_excluding(&[b"src/bulk/nested".to_vec()], &small, &excluded)[0]
            .problem,
        Some(Problem::Excluded)
    );
    assert_eq!(
        root.capture_selected(&[b"bulk/0".to_vec()], &small)[0]
            .bytes
            .as_deref(),
        Some(b"bulk".as_slice())
    );
    let via_api =
        snapshot_excluding(&root_path, Some(&[b"src/file".to_vec()]), &small, &excluded).unwrap();
    assert!(via_api.listing.complete);
    assert_eq!(
        via_api.captures[0].bytes.as_deref(),
        Some(b"wanted".as_slice())
    );
    let invalid = vec![b"bulk/child".to_vec()];
    let bad = root.observe_excluding(Some(&[b"src/file".to_vec()]), &small, &invalid);
    assert!(!bad.listing.complete);
    assert_eq!(bad.listing.issues[0].problem, Problem::InvalidPath);
    assert_eq!(bad.captures[0].problem, Some(Problem::InvalidPath));
}

mod membership;
mod races;
mod stream;
