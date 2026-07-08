use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub(super) fn cfg_test_module_file(root: &Path, rel: &str) -> bool {
    let mut seen = BTreeSet::new();
    cfg_test_module_file_inner(root, rel, &mut seen)
}

fn cfg_test_module_file_inner(root: &Path, rel: &str, seen: &mut BTreeSet<String>) -> bool {
    if rel.ends_with("_tests.rs") || rel.ends_with("/tests.rs") || rel.contains("/tests/") {
        return true;
    }
    if !seen.insert(rel.to_string()) {
        return false;
    }
    if cfg_test_path_owner_file(root, rel, seen) {
        return true;
    }
    let mut current_rel = rel.to_string();
    let mut conventional_seen = BTreeSet::new();
    while current_rel != "validator/src/lib.rs" && conventional_seen.insert(current_rel.clone()) {
        let Some((parent_rel, module_name)) = parent_module_and_name(&current_rel) else {
            return false;
        };
        if cfg_test_declared_in_parent_mod(root, &parent_rel, &module_name) {
            return true;
        }
        current_rel = parent_rel;
    }
    false
}

fn cfg_test_path_owner_file(root: &Path, rel: &str, seen: &mut BTreeSet<String>) -> bool {
    candidate_path_owner_files(root, rel)
        .into_iter()
        .any(|owner_rel| {
            if owner_rel == rel {
                return false;
            }
            let text = std::fs::read_to_string(root.join(&owner_rel)).unwrap_or_default();
            path_owner_declares_target(root, &owner_rel, &text, rel, seen)
        })
}

fn candidate_path_owner_files(root: &Path, rel: &str) -> Vec<String> {
    let Some((mut dir, _)) = rel.rsplit_once('/') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    loop {
        if let Ok(entries) = std::fs::read_dir(root.join(dir)) {
            out.extend(entries.filter_map(Result::ok).filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(|ext| ext.to_str()) == Some("rs")).then(|| {
                    let name = path.file_name()?.to_str()?;
                    Some(format!("{dir}/{name}"))
                })?
            }));
        }
        if dir == "validator/src" {
            break;
        }
        let Some((parent, _)) = dir.rsplit_once('/') else {
            break;
        };
        dir = parent;
    }
    out.sort();
    out.dedup();
    out
}

fn path_owner_declares_target(
    root: &Path,
    owner_rel: &str,
    text: &str,
    target_rel: &str,
    seen: &mut BTreeSet<String>,
) -> bool {
    let mut pending_cfg_test = false;
    let mut pending_path_rel = None::<String>;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if super::source::test_cfg_attribute(trimmed) {
            pending_cfg_test = true;
            pending_path_rel = None;
            continue;
        }
        if let Some(path_rel) = path_attribute_target(owner_rel, trimmed) {
            pending_path_rel = Some(path_rel);
            continue;
        }
        if pending_path_rel.as_deref() == Some(target_rel) && any_module_declaration(trimmed) {
            return pending_cfg_test || cfg_test_module_file_inner(root, owner_rel, seen);
        }
        if trimmed.starts_with("#[") {
            continue;
        }
        pending_cfg_test = false;
        pending_path_rel = None;
    }
    false
}

fn path_attribute_target(owner_rel: &str, trimmed: &str) -> Option<String> {
    let path = trimmed
        .strip_prefix("#[path = \"")
        .and_then(|rest| rest.strip_suffix("\"]"))?;
    let owner_dir = owner_rel.rsplit_once('/').map(|(dir, _)| dir)?;
    normalize_rel_path(Path::new(owner_dir).join(path))
}

fn normalize_rel_path(path: PathBuf) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

fn parent_module_and_name(rel: &str) -> Option<(String, String)> {
    let (dir, file_name) = rel.rsplit_once('/')?;
    if file_name == "mod.rs" {
        let (parent_dir, module_name) = dir.rsplit_once('/')?;
        let parent_rel = if parent_dir == "validator/src" {
            "validator/src/lib.rs".to_string()
        } else {
            format!("{parent_dir}/mod.rs")
        };
        Some((parent_rel, module_name.to_string()))
    } else {
        let module_name = file_name.strip_suffix(".rs")?.to_string();
        let parent_rel = if dir == "validator/src" {
            "validator/src/lib.rs".to_string()
        } else {
            format!("{dir}/mod.rs")
        };
        Some((parent_rel, module_name))
    }
}

fn cfg_test_declared_in_parent_mod(root: &Path, parent_rel: &str, module_name: &str) -> bool {
    let parent_text = std::fs::read_to_string(root.join(parent_rel)).unwrap_or_default();
    let mut pending_cfg_test = false;
    for line in parent_text.lines() {
        let trimmed = line.trim();
        if pending_cfg_test && module_declaration_for(trimmed, module_name) {
            return true;
        }
        if !trimmed.is_empty() {
            if super::source::test_cfg_attribute(trimmed) {
                pending_cfg_test = true;
            } else if pending_cfg_test && trimmed.starts_with("#[") {
                continue;
            } else {
                pending_cfg_test = false;
            }
        }
    }
    false
}

fn module_declaration_for(trimmed: &str, module_name: &str) -> bool {
    let trimmed = strip_visibility(trimmed);
    trimmed == format!("mod {module_name};")
        || trimmed.starts_with(&format!("mod {module_name} "))
        || trimmed.starts_with(&format!("mod {module_name}{{"))
}

fn any_module_declaration(trimmed: &str) -> bool {
    strip_visibility(trimmed).starts_with("mod ")
}

fn strip_visibility(trimmed: &str) -> &str {
    trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, text).expect("fixture file");
    }

    #[test]
    fn path_owner_resolution_fails_closed_for_cycles_and_malformed_labels() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "path-owner-cycle-fail-closed",
        );
        write(
            &root,
            "validator/src/observe/a.rs",
            "#[path = \"b.rs\"]\nmod b;\n",
        );
        write(
            &root,
            "validator/src/observe/b.rs",
            "#[path = \"a.rs\"]\nmod a;\n",
        );

        assert!(!cfg_test_module_file(&root, "validator/src/observe/a.rs"));
        assert!(!cfg_test_module_file(&root, "validator/src/invalid"));
        assert!(candidate_path_owner_files(&root, "lib.rs").is_empty());
        assert!(candidate_path_owner_files(&root, "observe/a.rs").is_empty());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn path_attribute_targets_are_normalized_and_absolute_paths_fail_closed() {
        assert_eq!(
            normalize_rel_path(PathBuf::from("./validator/src/observe.rs")),
            Some("validator/src/observe.rs".to_string())
        );
        assert_eq!(
            normalize_rel_path(PathBuf::from("validator/src/../observe.rs")),
            Some("validator/observe.rs".to_string())
        );
        assert_eq!(
            path_attribute_target("validator/src/owner.rs", "#[path = \"./target.rs\"]"),
            Some("validator/src/target.rs".to_string())
        );
        assert_eq!(
            path_attribute_target("validator/src/owner.rs", "#[path = \"/tmp/target.rs\"]"),
            None
        );
    }
}
