use crate::context::ReadSession;
use crate::inventory::digest::file_identity_regular;
use std::fs;
use std::path::Path;

pub(super) struct RetainedInspection {
    pub(super) replacement_targets: Vec<String>,
    pub(super) problems: Vec<(&'static str, &'static str)>,
}

fn problem(
    replacement_targets: Vec<String>,
    code: &'static str,
    message: &'static str,
) -> RetainedInspection {
    RetainedInspection {
        replacement_targets,
        problems: vec![(code, message)],
    }
}

pub(super) fn inspect(
    reads: &ReadSession,
    path: &Path,
    expected_sha256: &str,
    replacement_targets: &[String],
) -> RetainedInspection {
    let replacements = replacement_targets.to_vec();
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return problem(
            replacements,
            "retained_context_output_missing",
            "retained generated context output is missing",
        );
    };
    if metadata.file_type().is_symlink() {
        return problem(
            replacements,
            "retained_context_output_symlink",
            "retained generated context output must not be a symlink",
        );
    }
    if !metadata.is_file() {
        return problem(
            replacements,
            "retained_context_output_not_regular",
            "retained generated context output is not a regular file",
        );
    }
    let Ok((actual_sha256, _)) = file_identity_regular(reads, path) else {
        return problem(
            replacements,
            "retained_context_output_unreadable",
            "retained generated context output cannot be identity-bound",
        );
    };
    if actual_sha256 != expected_sha256 {
        return problem(
            replacements,
            "retained_context_digest_mismatch",
            "retained generated context output differs from its registry digest",
        );
    }
    RetainedInspection {
        replacement_targets: replacements,
        problems: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::inspect;
    use crate::context::{BuildRequest, LiveContext};
    use crate::inventory::digest::set_test_pauses;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn git(root: &Path, arguments: &[&str]) {
        assert!(
            Command::new("/usr/bin/git")
                .args(arguments)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn retained_context_path_swap_fails_identity_binding() {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-retained-race-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("generated")).unwrap();
        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "inventory@example.invalid"],
        );
        git(&root, &["config", "user.name", "Inventory Race Test"]);
        let target = root.join("generated/context.bin");
        let replacement = root.join("replacement.bin");
        fs::write(&target, vec![b'a'; 32 * 1024]).unwrap();
        fs::write(&replacement, vec![b'b'; 32 * 1024]).unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);

        let context = LiveContext::build(BuildRequest::new(&root)).unwrap();
        let reads = context.begin_read_session().unwrap();
        let swap_target = target.clone();
        set_test_pauses(150, 0);
        let swap = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            fs::rename(&swap_target, swap_target.with_extension("original")).unwrap();
            fs::rename(replacement, swap_target).unwrap();
        });
        let inspected = inspect(&reads, &target, &"a".repeat(64), &["HCT-CLAIMS".to_owned()]);
        swap.join().unwrap();
        assert_eq!(
            inspected.problems,
            [(
                "retained_context_output_unreadable",
                "retained generated context output cannot be identity-bound"
            )]
        );
        let _ = fs::remove_dir_all(root);
    }
}
