use super::{PackageCapture, PackageEntryKind, Repo, special_file_adapter};
use std::fs;

#[test]
fn snapshot_tree_preserves_symlink_special_and_hardlink_facts() {
    use std::os::unix::fs::symlink;

    let repo = Repo::new("package-snapshot-entry-kinds");
    fs::write(repo.root.join("ordinary.txt"), "ordinary").expect("ordinary");
    fs::hard_link(
        repo.root.join("ordinary.txt"),
        repo.root.join("hardlink.txt"),
    )
    .expect("hardlink");
    symlink("ordinary.txt", repo.root.join("symlink.txt")).expect("symlink");
    special_file_adapter::create_fifo(special_file_adapter::SpecialFileRequest {
        path: &repo.root.join("special.fifo"),
        mode: 0o600,
    })
    .expect("fifo");
    let context = repo.context();

    let snapshot = PackageCapture::begin(&context)
        .expect("snapshot capture")
        .finish()
        .expect("snapshot finish");
    assert_eq!(
        snapshot.tree().get("ordinary.txt"),
        Some(&PackageEntryKind::Regular { single_link: false })
    );
    assert_eq!(
        snapshot.tree().get("hardlink.txt"),
        Some(&PackageEntryKind::Regular { single_link: false })
    );
    assert_eq!(
        snapshot.tree().get("symlink.txt"),
        Some(&PackageEntryKind::Symlink)
    );
    assert_eq!(
        snapshot.tree().get("special.fifo"),
        Some(&PackageEntryKind::Special)
    );
}
