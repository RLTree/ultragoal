use super::Repo;
use crate::package::inventory::anchored::{Session, test_hooks};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn set_mode(path: &Path, mode: u32) {
    let mut permissions = fs::metadata(path).expect("mode metadata").permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions).expect("set mode");
}

fn assert_rejected_or_original(result: Result<(Vec<u8>, u32), String>, session: &Session) {
    if let Ok((bytes, mode)) = result {
        assert_eq!(bytes, b"trusted\n");
        assert_eq!(mode, 0o644, "replacement mode entered the source snapshot");
        assert!(
            session.finish().is_err(),
            "restored substitution escaped final descriptor reconciliation"
        );
    }
}

#[test]
fn packaged_modes_are_descriptor_bound_and_restored_substitutions_fail_closed() {
    let member = Repo::new("package-mode-member-substitution");
    let member_path = member.root.join("resource.txt");
    set_mode(&member_path, 0o644);
    let member_backup = member.root.join("resource.original");
    let mut session = Session::open(&member.root).expect("member session");
    session.seal_root_snapshot().expect("member root seal");
    let member_hook = Arc::new(AtomicBool::new(false));
    test_hooks::set_before_component("resource.txt", 0, {
        let member_path = member_path.clone();
        let member_backup = member_backup.clone();
        let member_hook = Arc::clone(&member_hook);
        move || {
            member_hook.store(true, Ordering::SeqCst);
            fs::rename(&member_path, &member_backup).expect("move member");
            fs::write(&member_path, b"replacement\n").expect("replacement member");
            set_mode(&member_path, 0o755);
            fs::remove_file(&member_path).expect("remove replacement member");
            fs::rename(&member_backup, &member_path).expect("restore member");
        }
    });
    let result = session.read_with_mode("resource.txt", 1024);
    assert!(
        member_hook.load(Ordering::SeqCst),
        "member hook did not run"
    );
    assert_rejected_or_original(result, &session);

    let ancestor = Repo::new("package-mode-ancestor-substitution");
    let skill_root = ancestor.root.join("skills/prove");
    fs::create_dir_all(&skill_root).expect("skill ancestor");
    let skill = skill_root.join("SKILL.md");
    fs::write(&skill, b"trusted\n").expect("skill source");
    set_mode(&skill, 0o644);
    let backup = ancestor.root.join("skills/prove.original");
    let mut session = Session::open(&ancestor.root).expect("ancestor session");
    session.seal_root_snapshot().expect("ancestor root seal");
    let ancestor_hook = Arc::new(AtomicBool::new(false));
    test_hooks::set_before_component("skills/prove/SKILL.md", 1, {
        let skill_root = skill_root.clone();
        let backup = backup.clone();
        let ancestor_hook = Arc::clone(&ancestor_hook);
        move || {
            ancestor_hook.store(true, Ordering::SeqCst);
            fs::rename(&skill_root, &backup).expect("move ancestor");
            fs::create_dir(&skill_root).expect("replacement ancestor");
            let replacement = skill_root.join("SKILL.md");
            fs::write(&replacement, b"replacement\n").expect("replacement skill");
            set_mode(&replacement, 0o755);
            fs::remove_dir_all(&skill_root).expect("remove replacement ancestor");
            fs::rename(&backup, &skill_root).expect("restore ancestor");
        }
    });
    let result = session.read_with_mode("skills/prove/SKILL.md", 1024);
    assert!(
        ancestor_hook.load(Ordering::SeqCst),
        "ancestor hook did not run"
    );
    assert_rejected_or_original(result, &session);

    let root = Repo::new("package-mode-root-substitution");
    let root_path = root.root.clone();
    let root_backup = root_path.with_extension("original-root");
    let mut session = Session::open(&root_path).expect("root session");
    session.seal_root_snapshot().expect("root seal");
    let root_hook = Arc::new(AtomicBool::new(false));
    test_hooks::set_before_component("resource.txt", 0, {
        let root_path = root_path.clone();
        let root_backup = root_backup.clone();
        let root_hook = Arc::clone(&root_hook);
        move || {
            root_hook.store(true, Ordering::SeqCst);
            fs::rename(&root_path, &root_backup).expect("move root");
            fs::create_dir(&root_path).expect("replacement root");
            let replacement = root_path.join("resource.txt");
            fs::write(&replacement, b"replacement\n").expect("replacement root member");
            set_mode(&replacement, 0o755);
            fs::remove_dir_all(&root_path).expect("remove replacement root");
            fs::rename(&root_backup, &root_path).expect("restore root");
        }
    });
    let result = session.read_with_mode("resource.txt", 1024);
    assert!(root_hook.load(Ordering::SeqCst), "root hook did not run");
    assert_rejected_or_original(result, &session);

    let capture_source = include_str!("../capture/mod.rs");
    assert!(capture_source.contains("read_with_mode"));
    assert!(!capture_source.contains("symlink_metadata"));
    assert!(!capture_source.contains("root.join(path)"));
}
