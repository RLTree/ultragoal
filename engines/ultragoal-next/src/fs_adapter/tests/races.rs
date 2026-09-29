use super::*;

#[test]
fn leaf_and_parent_substitution_are_unstable() {
    let tree = Tree::new();
    let outside = Tree::new();
    fs::write(tree.at("leaf"), b"original").unwrap();
    fs::write(outside.at("secret"), b"secret").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let leaf = root.capture_with_hook(b"leaf", 1024, 1024, |phase, _| {
        if matches!(phase, Phase::AfterOpen) {
            fs::rename(tree.at("leaf"), tree.at("old")).unwrap();
            symlink(outside.at("secret"), tree.at("leaf")).unwrap();
        }
    });
    assert_eq!(leaf.problem, Some(Problem::Unstable));
    assert!(leaf.bytes.is_none());
    fs::create_dir(tree.at("parent_dir")).unwrap();
    fs::write(tree.at("parent_dir/file"), b"inside").unwrap();
    let parent = root.capture_with_hook(b"parent_dir/file", 1024, 1024, |phase, _| {
        if matches!(phase, Phase::AfterOpen) {
            fs::rename(tree.at("parent_dir"), tree.at("old_parent")).unwrap();
            symlink(&outside.0, tree.at("parent_dir")).unwrap();
        }
    });
    assert_eq!(parent.problem, Some(Problem::Unstable));
    assert!(parent.bytes.is_none());
    assert_eq!(fs::read(outside.at("secret")).unwrap(), b"secret");
}

#[test]
fn nofollow_blocks_leaf_replacement_between_type_check_and_open() {
    let tree = Tree::new();
    let outside = Tree::new();
    fs::write(tree.at("file"), b"inside").unwrap();
    fs::write(outside.at("secret"), b"outside").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let result = root.capture_with_hook(b"file", 1024, 1024, |phase, _| {
        if matches!(phase, Phase::AfterTypeCheck) {
            fs::rename(tree.at("file"), tree.at("old")).unwrap();
            symlink(outside.at("secret"), tree.at("file")).unwrap();
        }
    });
    assert!(matches!(result.problem, Some(Problem::Unavailable(_))));
    assert!(result.bytes.is_none());
    assert_eq!(fs::read(outside.at("secret")).unwrap(), b"outside");
}

#[test]
fn root_open_rejects_symlink_in_absolute_parent_chain() {
    let tree = Tree::new();
    fs::create_dir(tree.at("real")).unwrap();
    fs::create_dir(tree.at("real/child")).unwrap();
    symlink(tree.at("real"), tree.at("alias")).unwrap();
    let canonical_parent = fs::canonicalize(&tree.0).unwrap();
    assert!(Root::open(&canonical_parent.join("alias")).is_err());
    assert!(Root::open(&canonical_parent.join("alias").join("child")).is_err());
    assert!(Root::open(&canonical_parent.join("real")).is_ok());
}

#[test]
fn snapshot_rejects_a_stable_replacement_after_enumeration() {
    let tree = Tree::new();
    fs::write(tree.at("file"), b"same").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let selected = [b"file".to_vec()];
    let snapshot = root.observe_with_hook(Some(&selected), &limits(), |_| {
        fs::rename(tree.at("file"), tree.at("old")).unwrap();
        fs::write(tree.at("file"), b"same").unwrap();
    });
    assert!(snapshot.listing.complete);
    assert_eq!(snapshot.captures[0].problem, Some(Problem::Unstable));
    assert!(snapshot.captures[0].bytes.is_none());
    assert_eq!(fs::read(tree.at("file")).unwrap(), b"same");
}

#[test]
fn root_path_replacement_does_not_renew_old_descriptor_authority() {
    let tree = Tree::new();
    fs::write(tree.at("file"), b"old-root").unwrap();
    let canonical = fs::canonicalize(&tree.0).unwrap();
    let root = Root::open(&canonical).unwrap();
    let moved = canonical.with_extension("moved");
    fs::rename(&canonical, &moved).unwrap();
    fs::create_dir(&canonical).unwrap();
    fs::write(canonical.join("file"), b"new-root").unwrap();
    let capture = root.capture_selected(&[b"file".to_vec()], &limits());
    assert_eq!(capture[0].problem, Some(Problem::Unstable));
    assert!(capture[0].bytes.is_none());
    assert_eq!(fs::read(canonical.join("file")).unwrap(), b"new-root");
    fs::remove_dir_all(moved).unwrap();
}

#[test]
fn partial_read_and_changed_then_restored_bytes_are_unstable() {
    let tree = Tree::new();
    fs::write(tree.at("large"), vec![b'a'; 131_072]).unwrap();
    fs::write(tree.at("restore"), b"original").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let partial = root.capture_with_hook(b"large", 1024, 200_000, |phase, count| {
        if matches!(phase, Phase::AfterChunk) && count == 65_536 {
            OpenOptions::new()
                .write(true)
                .open(tree.at("large"))
                .unwrap()
                .set_len(65_536)
                .unwrap();
        }
    });
    assert_eq!(partial.problem, Some(Problem::Unstable));
    assert!(partial.bytes.is_none());
    let restored = root.capture_with_hook(b"restore", 1024, 1024, |phase, _| {
        if matches!(phase, Phase::BeforeRevalidate) {
            fs::write(tree.at("restore"), b"modified").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
            fs::write(tree.at("restore"), b"original").unwrap();
        }
    });
    assert_eq!(restored.problem, Some(Problem::Unstable));
    assert!(restored.bytes.is_none());
    assert_eq!(fs::read(tree.at("restore")).unwrap(), b"original");
}

#[test]
fn bounds_preserve_independent_results_and_fds_close() {
    let tree = Tree::new();
    fs::write(tree.at("a"), b"one").unwrap();
    fs::write(tree.at("b"), b"two").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let mut small = limits();
    small.max_total_bytes = 3;
    let captures = root.capture_selected(&[b"a".to_vec(), b"b".to_vec()], &small);
    assert_eq!(captures[0].bytes.as_deref(), Some(b"one".as_slice()));
    assert_eq!(captures[1].problem, Some(Problem::Limit));
    assert!(captures[1].bytes.is_none());
    small.max_entries = 1;
    let listing = root.enumerate(&small);
    assert!(!listing.complete);
    assert_eq!(listing.members.len(), 1);
    assert!(
        listing
            .issues
            .iter()
            .any(|item| item.problem == Problem::EntryBound)
    );
    let selected = root.capture_selected(&[b"a".to_vec(), b"b".to_vec()], &small);
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[1].path, b""); // Batch-wide truncation marker.
    assert_eq!(selected[1].problem, Some(Problem::Limit));
    let mut file_limit = limits();
    file_limit.max_file_bytes = 2;
    assert_eq!(
        root.capture_selected(&[b"a".to_vec()], &file_limit)[0].problem,
        Some(Problem::Limit)
    );
    let mut path_limit = limits();
    path_limit.max_path_bytes = 0;
    assert!(!root.enumerate(&path_limit).complete);
    assert_eq!(
        root.capture_selected(&[b"a".to_vec()], &path_limit)[0].problem,
        Some(Problem::Limit)
    );
    let fd = root.directory.as_raw_fd();
    let identity = |fd| {
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        (unsafe { libc::fstat(fd, &mut st) } == 0).then_some((st.st_dev, st.st_ino))
    };
    let before = identity(fd);
    assert!(before.is_some());
    drop(root);
    // Parallel tests may reuse the descriptor number, so check that it no longer
    // refers to the root directory rather than that the number is unused.
    assert_ne!(identity(fd), before);
}

fn window_of(root: &Root, parent: &str, leaves: &[&'static str], hook: &mut dyn FnMut(Phase, usize)) -> Vec<Capture> {
    let parent: Vec<std::ffi::CString> = if parent.is_empty() { vec![] } else { vec![std::ffi::CString::new(parent).unwrap()] };
    let paths: Vec<Vec<u8>> = leaves.iter().map(|l| if parent.is_empty() { l.as_bytes().to_vec() } else { format!("{}/{l}", parent[0].to_str().unwrap()).into_bytes() }).collect();
    let files: Vec<(&[u8], std::ffi::CString)> = paths.iter().zip(leaves).map(|(p, l)| (p.as_slice(), std::ffi::CString::new(*l).unwrap())).collect();
    root.capture_window(&parent, &files, 1024, &mut |phase, n| hook(phase, n))
}

#[test]
fn window_rechecks_every_name_after_the_last_read() {
    let tree = Tree::new();
    fs::create_dir(tree.at("d")).unwrap();
    for name in ["a", "b", "c"] {
        fs::write(tree.at(&format!("d/{name}")), name.as_bytes()).unwrap();
    }
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let steady = window_of(&root, "d", &["a", "b", "c"], &mut |_, _| {});
    assert!(steady.iter().all(|c| c.problem.is_none()));
    // A file already read is rewritten and restored while a later file is read.
    let mut opened = 0;
    let rewritten = window_of(&root, "d", &["a", "b", "c"], &mut |phase, _| {
        if matches!(phase, Phase::AfterOpen) {
            opened += 1;
            if opened == 2 {
                fs::write(tree.at("d/a"), b"x").unwrap();
                std::thread::sleep(std::time::Duration::from_millis(2));
                fs::write(tree.at("d/a"), b"a").unwrap();
            }
        }
    });
    assert_eq!(rewritten[0].problem, Some(Problem::Unstable));
    assert!(rewritten[0].bytes.is_none());
    assert!(rewritten[1].problem.is_none() && rewritten[2].problem.is_none());
    // Replacing a name changes the directory's identity, so the whole window is
    // rejected, not only the replaced file.
    let replaced = window_of(&root, "d", &["a", "b", "c"], &mut |phase, _| {
        if matches!(phase, Phase::BeforeRevalidate) {
            fs::rename(tree.at("d/b"), tree.at("d/old-b")).unwrap();
            fs::write(tree.at("d/b"), b"b").unwrap();
        }
    });
    assert!(replaced.iter().all(|c| c.problem == Some(Problem::Unstable) && c.bytes.is_none()));
}

#[test]
fn window_parent_substitution_rejects_the_whole_window() {
    let tree = Tree::new();
    let outside = Tree::new();
    fs::create_dir(tree.at("d")).unwrap();
    fs::write(tree.at("d/a"), b"a").unwrap();
    fs::write(tree.at("d/b"), b"b").unwrap();
    fs::write(outside.at("a"), b"a").unwrap();
    fs::write(outside.at("b"), b"b").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let result = window_of(&root, "d", &["a", "b"], &mut |phase, _| {
        if matches!(phase, Phase::BeforeRevalidate) {
            fs::rename(tree.at("d"), tree.at("old-d")).unwrap();
            symlink(&outside.0, tree.at("d")).unwrap();
        }
    });
    assert!(result.iter().all(|c| c.problem == Some(Problem::Unstable) && c.bytes.is_none()));
}
