use crate::audit::contract::Failure;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
pub(super) fn compare_records(left: &Value, right: &Value, root: &str, out: &mut Vec<Failure>) {
    for key in ["lease_id", "lane_id", "branch", "worktree"] {
        if let (Some(a), Some(b)) = (
            left.get(key).and_then(Value::as_str),
            right.get(key).and_then(Value::as_str),
        ) {
            if a == b {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_identity_overlap",
                    key,
                ));
            }
        }
    }
    for key in ["owned_symbols", "effects"] {
        let overlap = array_set(left, key)
            .intersection(&array_set(right, key))
            .next()
            .cloned();
        if let Some(item) = overlap {
            out.push(Failure::new(
                "authority-lease",
                "lease_surface_overlap",
                format!("{key}:{item}"),
            ));
        }
    }
    let left_paths = normalized_union(left, root);
    let right_paths = normalized_union(right, root);
    if let Some(item) = left_paths
        .iter()
        .find(|left| right_paths.iter().any(|right| paths_conflict(left, right)))
    {
        out.push(Failure::new(
            "authority-lease",
            "lease_path_surface_overlap",
            item,
        ));
    }
}
fn paths_conflict(left: &str, right: &str) -> bool {
    let (left, right) = (left.to_ascii_lowercase(), right.to_ascii_lowercase());
    left == right
        || left.starts_with(&format!("{right}/"))
        || right.starts_with(&format!("{left}/"))
}
pub(super) fn normalize_path(_root: &Path, authority: &str, raw: &str) -> Option<String> {
    let relative = Path::new(raw);
    if authority.is_empty()
        || relative
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    let authority = Path::new(authority);
    let candidate = if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        authority.join(relative)
    };
    let mut ancestor = candidate.clone();
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(ancestor.file_name()?.to_owned());
        ancestor.pop();
    }
    let canonical = ancestor.canonicalize().ok()?;
    let canonical_authority = authority.canonicalize().ok()?;
    if !canonical.starts_with(&canonical_authority) {
        return None;
    }
    #[cfg(unix)]
    if canonical.is_file() && canonical.metadata().ok()?.nlink() > 1 {
        return None;
    }
    let mut result = PathBuf::from(canonical);
    for component in suffix.iter().rev() {
        result.push(component);
    }
    Some(result.to_string_lossy().into_owned())
}
pub(super) fn surface_union(record: &Value, root: &Path, authority: &str, out: &mut Vec<Failure>) {
    let mut seen = BTreeSet::new();
    for key in [
        "owned_files",
        "generated_outputs",
        "fixtures",
        "isolated_roots",
    ] {
        for raw in array_set(record, key) {
            let Some(path) = normalize_path(root, authority, &raw) else {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_path_uncontained",
                    raw,
                ));
                continue;
            };
            if !seen.insert(path.clone()) {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_cross_category_overlap",
                    path,
                ));
            }
        }
    }
    for key in ["owned_symbols", "effects"] {
        if let Some(values) = record.get(key).and_then(Value::as_array) {
            for raw in values.iter().filter_map(Value::as_str) {
                let namespace = if key == "effects" && (raw.starts_with('/') || raw.contains('/')) {
                    let Some(path) = normalize_path(root, authority, raw) else {
                        out.push(Failure::new(
                            "authority-lease",
                            "lease_effect_uncontained",
                            raw,
                        ));
                        continue;
                    };
                    path
                } else {
                    raw.trim().to_owned()
                };
                if !seen.insert(format!("{key}:{namespace}")) {
                    out.push(Failure::new(
                        "authority-lease",
                        "lease_namespace_duplicate",
                        namespace,
                    ));
                }
            }
        }
    }
}
fn normalized_union(record: &Value, authority: &str) -> BTreeSet<String> {
    let mut result = [
        "owned_files",
        "generated_outputs",
        "fixtures",
        "isolated_roots",
    ]
    .into_iter()
    .flat_map(|key| array_set(record, key))
    .filter_map(|path| normalize_path(Path::new("/"), authority, &path))
    .collect::<BTreeSet<_>>();
    if let Some(values) = record.get("effects").and_then(Value::as_array) {
        for raw in values
            .iter()
            .filter_map(Value::as_str)
            .filter(|raw| raw.starts_with('/') || raw.contains('/'))
        {
            if let Some(path) = normalize_path(Path::new("/"), authority, raw) {
                result.insert(path);
            }
        }
    }
    if let Some(worktree) = record.get("worktree").and_then(Value::as_str) {
        if let Some(path) = normalize_path(Path::new("/"), authority, worktree) {
            result.insert(path);
        }
    }
    result
}
pub(super) fn check_protected(
    record: &Value,
    root: &Path,
    authority: &str,
    out: &mut Vec<Failure>,
) {
    let patterns = array_set(record, "forbidden_roots")
        .into_iter()
        .chain(array_set(record, "root_only_surfaces"))
        .chain(array_set(record, "protected_root_patterns"))
        .collect::<Vec<_>>();
    for key in [
        "owned_files",
        "generated_outputs",
        "fixtures",
        "isolated_roots",
    ] {
        for raw in array_set(record, key) {
            let Some(normalized) = normalize_path(root, authority, &raw) else {
                continue;
            };
            let alias = raw
                .split('/')
                .any(|part| matches!(part, ".git" | ".codex" | ".agents"));
            let protected = patterns
                .iter()
                .any(|pattern| super::protected_path_match(&normalized, pattern));
            if alias || protected {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_protected_surface_owned",
                    format!("{key}:{raw}"),
                ));
            }
        }
    }
    if let Some(values) = record.get("effects").and_then(Value::as_array) {
        for raw in values
            .iter()
            .filter_map(Value::as_str)
            .filter(|raw| raw.starts_with('/') || raw.contains('/'))
        {
            let alias = raw
                .split('/')
                .any(|part| matches!(part, ".git" | ".codex" | ".agents"));
            let protected = normalize_path(root, authority, raw).is_some_and(|path| {
                patterns
                    .iter()
                    .any(|pattern| super::protected_path_match(&path, pattern))
            });
            if alias || protected {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_protected_effect",
                    raw,
                ));
            }
        }
    }
}
pub(super) fn array_set(record: &Value, key: &str) -> BTreeSet<String> {
    let values = match record.get(key) {
        Some(Value::Array(values)) => values.iter().collect::<Vec<_>>(),
        Some(Value::Object(values)) => values.values().collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    values
        .into_iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
