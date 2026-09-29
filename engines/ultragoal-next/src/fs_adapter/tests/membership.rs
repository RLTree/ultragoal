use super::*;
use std::time::{Duration, Instant};

fn complete(root: &Root) -> MembershipCheckpoint {
    root.membership_checkpoint(&limits()).unwrap()
}

fn checked(root: &Root, checkpoint: &MembershipCheckpoint) -> usize {
    match root
        .revalidate_membership(checkpoint, &limits(), &[])
        .unwrap()
    {
        MembershipObservation::DirectoryIdentitiesUnchanged {
            checked_directories,
        } => checked_directories,
    }
}

#[test]
fn unchanged_directories_revalidate_without_leaf_enumeration_or_content_claim() {
    let tree = Tree::new();
    fs::create_dir(tree.at("empty")).unwrap();
    fs::create_dir(tree.at("sub")).unwrap();
    fs::write(tree.at("sub/selected"), b"first").unwrap();
    fs::write(tree.at("sub/unselected"), b"other").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let checkpoint = complete(&root);
    assert_eq!(checkpoint.listing().members.len(), 4);
    let calls_before = os::readdir_calls();
    assert_eq!(checked(&root, &checkpoint), 3); // Root and both included directories.
    assert_eq!(os::readdir_calls(), calls_before);

    fs::write(tree.at("sub/unselected"), b"changed-content").unwrap();
    assert_eq!(checked(&root, &checkpoint), 3);
    fs::write(tree.at("sub/selected"), b"second").unwrap();
    assert_eq!(checked(&root, &checkpoint), 3);
    assert_eq!(os::readdir_calls(), calls_before);
    let selected = root.capture_selected(&[b"sub/selected".to_vec()], &limits());
    assert_eq!(selected[0].bytes.as_deref(), Some(b"second".as_slice()));
    assert_ne!(
        selected[0].identity,
        checkpoint
            .listing()
            .members
            .iter()
            .find(|member| member.path == b"sub/selected")
            .map(|member| member.identity)
    );
}

#[test]
fn additions_deletions_and_fast_add_delete_require_fallback() {
    let tree = Tree::new();
    fs::create_dir(tree.at("empty")).unwrap();
    fs::write(tree.at("keep"), b"content").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let baseline = complete(&root);
    fs::write(tree.at("empty/added"), b"a").unwrap();
    assert!(
        matches!(root.revalidate_membership(&baseline, &limits(), &[]),
        Err(MembershipFallback::DirectoryChanged { path }) if path == b"empty")
    );
    fs::remove_file(tree.at("empty/added")).unwrap();
    let after_add = complete(&root);
    fs::remove_file(tree.at("keep")).unwrap();
    assert!(matches!(
        root.revalidate_membership(&after_add, &limits(), &[]),
        Err(MembershipFallback::RootChanged)
    ));

    let ab_tree = Tree::new();
    fs::create_dir(ab_tree.at("empty")).unwrap();
    let ab_root = Root::open(&fs::canonicalize(&ab_tree.0).unwrap()).unwrap();
    let ab = complete(&ab_root);
    // A transient add/delete leaves the name set unchanged. Its directory
    // metadata changes on this host; this is an observed ABA case, not a proof
    // that every filesystem or adversarial timestamp restoration is detected.
    fs::write(ab_tree.at("empty/transient"), b"x").unwrap();
    std::thread::sleep(Duration::from_millis(2));
    fs::remove_file(ab_tree.at("empty/transient")).unwrap();
    assert!(matches!(ab_root.revalidate_membership(&ab, &limits(), &[]),
        Err(MembershipFallback::DirectoryChanged { path }) if path == b"empty"));
}

#[test]
fn directory_and_root_replacements_require_fallback() {
    let tree = Tree::new();
    fs::create_dir(tree.at("sub")).unwrap();
    fs::write(tree.at("sub/file"), b"x").unwrap();
    let canonical = fs::canonicalize(&tree.0).unwrap();
    let root = Root::open(&canonical).unwrap();
    let checkpoint = complete(&root);
    fs::rename(tree.at("sub"), tree.at("old_sub")).unwrap();
    fs::create_dir(tree.at("sub")).unwrap();
    fs::write(tree.at("sub/file"), b"x").unwrap();
    assert!(matches!(
        root.revalidate_membership(&checkpoint, &limits(), &[]),
        Err(MembershipFallback::RootChanged) | Err(MembershipFallback::DirectoryChanged { .. })
    ));

    let checkpoint = complete(&root);
    let moved = canonical.with_extension("moved");
    fs::rename(&canonical, &moved).unwrap();
    fs::create_dir(&canonical).unwrap();
    assert!(matches!(
        root.revalidate_membership(&checkpoint, &limits(), &[]),
        Err(MembershipFallback::RootChanged)
    ));
    fs::remove_dir_all(moved).unwrap();
}

#[test]
fn scope_partial_listing_and_closed_descriptors() {
    let tree = Tree::new();
    fs::create_dir(tree.at("skip")).unwrap();
    fs::write(tree.at("skip/a"), b"a").unwrap();
    fs::write(tree.at("keep"), b"b").unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let excluded = [b"skip".to_vec()];
    let checkpoint = root
        .membership_checkpoint_excluding(&limits(), &excluded)
        .unwrap();
    assert_eq!(checkpoint.listing().excluded_basenames, excluded);
    assert_eq!(checked_with(&root, &checkpoint, &limits(), &excluded), 1);
    fs::write(tree.at("skip/new"), b"out-of-universe").unwrap();
    assert_eq!(checked_with(&root, &checkpoint, &limits(), &excluded), 1);
    assert!(matches!(
        root.revalidate_membership(&checkpoint, &limits(), &[]),
        Err(MembershipFallback::ScopeChanged)
    ));
    let mut changed_limits = limits();
    changed_limits.max_entries -= 1;
    assert!(matches!(
        root.revalidate_membership(&checkpoint, &changed_limits, &excluded),
        Err(MembershipFallback::ScopeChanged)
    ));
    let mut partial_limits = limits();
    partial_limits.max_entries = 1;
    assert!(matches!(root.membership_checkpoint(&partial_limits),
        Err(MembershipFallback::InitialListingIncomplete(listing)) if !listing.complete));
    let invalid = [b"skip/child".to_vec()];
    assert!(
        matches!(root.membership_checkpoint_excluding(&limits(), &invalid),
        Err(MembershipFallback::InitialListingIncomplete(listing)) if !listing.complete)
    );

    for _ in 0..64 {
        assert_eq!(checked_with(&root, &checkpoint, &limits(), &excluded), 1);
    }
    let fd = root.directory.as_raw_fd();
    assert!(unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0);
    drop(root);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
}

#[test]
fn inaccessible_initial_subdirectory_cannot_create_checkpoint() {
    let tree = Tree::new();
    fs::create_dir(tree.at("locked")).unwrap();
    fs::set_permissions(tree.at("locked"), fs::Permissions::from_mode(0o0)).unwrap();
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    assert!(matches!(root.membership_checkpoint(&limits()),
        Err(MembershipFallback::InitialListingIncomplete(listing))
            if !listing.complete && listing.issues.iter().any(|issue| issue.path == b"locked")));
    fs::set_permissions(tree.at("locked"), fs::Permissions::from_mode(0o700)).unwrap();
}

fn checked_with(
    root: &Root,
    checkpoint: &MembershipCheckpoint,
    limits: &Limits,
    excluded: &[Vec<u8>],
) -> usize {
    match root
        .revalidate_membership(checkpoint, limits, excluded)
        .unwrap()
    {
        MembershipObservation::DirectoryIdentitiesUnchanged {
            checked_directories,
        } => checked_directories,
    }
}

#[test]
fn small_fixture_measurement_reports_actual_work_only() {
    let tree = Tree::new();
    for dir in 0..4 {
        fs::create_dir(tree.at(&format!("d{dir}"))).unwrap();
        for file in 0..64 {
            fs::write(tree.at(&format!("d{dir}/f{file}")), b"x").unwrap();
        }
    }
    let root = Root::open(&fs::canonicalize(&tree.0).unwrap()).unwrap();
    let mut measure_limits = limits();
    measure_limits.max_entries = 300;
    let start = Instant::now();
    let checkpoint = root.membership_checkpoint(&measure_limits).unwrap();
    let scan_elapsed = start.elapsed();
    let calls_before = os::readdir_calls();
    let start = Instant::now();
    let count = checked_with(&root, &checkpoint, &measure_limits, &[]);
    let revalidate_elapsed = start.elapsed();
    assert_eq!(count, 5);
    assert_eq!(checkpoint.listing().members.len(), 260);
    assert_eq!(os::readdir_calls(), calls_before);
    eprintln!(
        "membership fixture: 260 entries, 5 directories, initial={scan_elapsed:?}, revalidate={revalidate_elapsed:?}, readdir_delta=0"
    );
}
