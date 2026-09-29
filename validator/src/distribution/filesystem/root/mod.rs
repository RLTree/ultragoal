use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::{sha256, validate_relative_path};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use super::descriptor::{Directory, DirectoryIdentity, EntryKind};

include!("confined_root.rs");
include!("temporary_parent.rs");
include!("workspace_context.rs");

#[cfg(all(test, unix))]
mod cleanup_tests;

#[cfg(all(test, unix))]
mod tests {
    use super::ConfinedRoot;
    use crate::distribution::DistributionErrorId;
    use crate::distribution::filesystem::file::ScopedFile;
    use crate::distribution::filesystem::hooks::{
        EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
    };
    use std::ffi::CString;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot {
        path: PathBuf,
        original_gid: u32,
    }

    impl TestRoot {
        fn new(label: &str, mode: u32) -> Self {
            let path = std::env::temp_dir().join(format!(
                "hul-distribution-root-authority-{label}-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let original_gid = fs::symlink_metadata(&path).unwrap().gid();
            Self { path, original_gid }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn set_mode(&self, mode: u32) {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(mode)).unwrap();
        }

        fn set_gid(&self, gid: u32) -> bool {
            let path = CString::new(self.path.as_os_str().as_bytes()).unwrap();
            (unsafe { libc::chown(path.as_ptr(), !0 as libc::uid_t, gid as libc::gid_t) }) == 0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = self.set_gid(self.original_gid);
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700));
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn owned_non_writable_root_remains_usable() {
        let fixture = TestRoot::new("positive", 0o755);
        let root = ConfinedRoot::open(fixture.path()).unwrap();
        let output = ScopedFile::new(root, "state/value.json").unwrap();

        assert!(output.apply(None, Some(b"bound-root")).unwrap());
        assert_eq!(output.inspect(1024).unwrap(), Some(b"bound-root".to_vec()));
    }

    #[test]
    fn group_or_other_writable_root_is_rejected_at_open() {
        let fixture = TestRoot::new("unsafe-open", 0o777);

        assert_eq!(
            ConfinedRoot::open(fixture.path()).unwrap_err().id(),
            DistributionErrorId::UnsafeObject
        );
    }

    #[test]
    fn retained_root_mode_drift_fails_before_publication() {
        let fixture = TestRoot::new("mode-drift", 0o700);
        let root = ConfinedRoot::open(fixture.path()).unwrap();
        let output = ScopedFile::new(root, "state/value.json").unwrap();
        fixture.set_mode(0o777);

        assert_eq!(
            output
                .apply(None, Some(b"must-not-publish"))
                .unwrap_err()
                .id(),
            DistributionErrorId::ObjectChanged
        );
        assert!(!fixture.path().join("state/value.json").exists());
        assert!(fs::read_dir(fixture.path()).unwrap().next().is_none());
    }

    #[test]
    fn retained_root_mode_drift_at_effect_boundary_fails_before_publication() {
        let fixture = TestRoot::new("effect-boundary", 0o700);
        let root = ConfinedRoot::open(fixture.path()).unwrap();
        let output = ScopedFile::new(root, "state/value.json").unwrap();
        let root_path = fixture.path().to_path_buf();
        set_test_effect_hook_matching(EffectPoint::CreateFile, ".hul-lock-", move |_| {
            fs::set_permissions(&root_path, fs::Permissions::from_mode(0o777)).unwrap();
        });

        assert_eq!(
            output
                .apply(None, Some(b"must-not-publish"))
                .unwrap_err()
                .id(),
            DistributionErrorId::ObjectChanged
        );
        assert_test_effect_hook_consumed();
        assert!(!fixture.path().join("state/value.json").exists());
        assert!(fs::read_dir(fixture.path()).unwrap().next().is_none());
    }

    #[test]
    fn retained_root_group_drift_fails_when_a_second_group_is_available() {
        let fixture = TestRoot::new("group-drift", 0o700);
        let root = ConfinedRoot::open(fixture.path()).unwrap();
        let current_gid = fs::symlink_metadata(fixture.path()).unwrap().gid();
        let Some(alternate_gid) = supplementary_groups()
            .into_iter()
            .find(|gid| *gid != current_gid)
        else {
            return;
        };
        if !fixture.set_gid(alternate_gid) {
            return;
        }

        assert_eq!(
            root.revalidate().unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
    }

    #[test]
    fn workspace_open_rejects_same_candidate_root_substitution() {
        let parent = std::env::temp_dir().join(format!(
            "hul-distribution-workspace-swap-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        let root = parent.join("worktree");
        let replacement = parent.join("replacement");
        let original = parent.join("original");
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(
            &root,
            &["config", "user.email", "distribution@example.invalid"],
        );
        run_git(&root, &["config", "user.name", "Distribution Test"]);
        fs::write(root.join("tracked.txt"), b"same candidate\n").unwrap();
        run_git(&root, &["add", "tracked.txt"]);
        run_git(&root, &["commit", "-qm", "fixture"]);
        let clone = Command::new("git")
            .args(["clone", "-q", "--no-hardlinks", "--"])
            .arg(&root)
            .arg(&replacement)
            .status()
            .unwrap();
        assert!(clone.success());
        let context = crate::context::LiveContext::build(
            crate::context::BuildRequest::new(&root).with_root_workspace_grant(&root),
        )
        .unwrap();
        let root_for_swap = root.clone();
        let replacement_for_swap = replacement.clone();
        let original_for_swap = original.clone();
        set_test_effect_hook_matching(EffectPoint::OpenDirectory, "worktree", move |_| {
            fs::rename(&root_for_swap, &original_for_swap).unwrap();
            fs::rename(&replacement_for_swap, &root_for_swap).unwrap();
        });

        let failure = ConfinedRoot::open_workspace(&context).unwrap_err();

        assert_test_effect_hook_consumed();
        assert_eq!(failure.id(), DistributionErrorId::ObjectChanged);
        assert!(
            !root
                .join("target/ultragoal/package-inventory.json")
                .exists()
        );
        fs::remove_dir_all(parent).unwrap();
    }

    fn run_git(root: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }

    fn supplementary_groups() -> Vec<u32> {
        let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        if count <= 0 {
            return Vec::new();
        }
        let mut groups = vec![0 as libc::gid_t; count as usize];
        let written = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
        if written < 0 {
            return Vec::new();
        }
        groups
            .into_iter()
            .take(written as usize)
            .map(|gid| gid as u32)
            .collect()
    }
}
