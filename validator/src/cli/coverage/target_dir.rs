use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetDirStatus {
    Missing,
    Isolated,
    NotIsolated,
}

pub(crate) fn status(root: &Path, raw: &str) -> TargetDirStatus {
    if raw.trim().is_empty() {
        return TargetDirStatus::Missing;
    }
    let root = normalize_path(root);
    let target = normalize_path(root.join("target"));
    let candidate = if Path::new(raw).is_absolute() {
        normalize_path(raw)
    } else {
        normalize_path(root.join(raw))
    };
    if candidate == target || canonical_matches_target(&candidate, &target) {
        TargetDirStatus::NotIsolated
    } else {
        TargetDirStatus::Isolated
    }
}

fn canonical_matches_target(candidate: &Path, target: &Path) -> bool {
    let Ok(candidate) = candidate.canonicalize() else {
        return false;
    };
    let Ok(target) = target.canonicalize() else {
        return false;
    };
    candidate == target
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            _ => out.push(component.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{TargetDirStatus, status};

    #[test]
    fn target_dir_status_rejects_target_aliases() {
        let root = std::env::current_dir().expect("cwd");
        for raw in [
            "target",
            "./target",
            "target/",
            "target/.",
            "target/../target",
        ] {
            assert_eq!(status(&root, raw), TargetDirStatus::NotIsolated, "{raw}");
        }
        assert_eq!(
            status(&root, &root.join("target").display().to_string()),
            TargetDirStatus::NotIsolated
        );
    }

    #[test]
    fn target_dir_status_accepts_named_isolated_child() {
        let root = std::env::current_dir().expect("cwd");
        assert_eq!(
            status(&root, "target/ultragoal-coverage"),
            TargetDirStatus::Isolated
        );
    }

    #[test]
    fn target_dir_status_accepts_existing_non_target_when_target_is_absent() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "coverage-target-dir-without-target",
        );
        let isolated = root.join("coverage-output");
        std::fs::create_dir_all(&isolated).expect("isolated target");

        assert_eq!(
            status(&root, isolated.to_str().expect("utf8 isolated target")),
            TargetDirStatus::Isolated
        );

        std::fs::remove_dir_all(root).expect("cleanup target dir status");
    }
}
