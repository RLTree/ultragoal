use crate::digest;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) struct SkillLinkFailure {
    pub(crate) code: &'static str,
    pub(crate) detail: String,
}

pub(crate) fn manifest_failures(root: &Path, manifest: &Value) -> Vec<SkillLinkFailure> {
    manifest
        .get("skills")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|row| skill_failures(root, row))
        .collect()
}

fn skill_failures(root: &Path, row: &Value) -> Vec<SkillLinkFailure> {
    let rel = row.get("path").and_then(Value::as_str).unwrap_or("");
    let path = match crate::package::inventory::resolve(root, rel) {
        Ok(path) if path.is_file() => path,
        _ => return Vec::new(),
    };
    let text = match digest::read_file_bytes(&path)
        .and_then(|bytes| String::from_utf8(bytes).map_err(|err| format!("skill file utf8: {err}")))
    {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let skill_dir = path.parent().unwrap_or(root);
    extract_refs(&text)
        .into_iter()
        .filter_map(|raw| classify_ref(root, skill_dir, rel, &raw))
        .collect()
}

fn classify_ref(
    root: &Path,
    skill_dir: &Path,
    skill_rel: &str,
    raw: &str,
) -> Option<SkillLinkFailure> {
    let rel = normalized_ref(raw)?;
    if resolves_inside(root, &skill_dir.join(&rel)) {
        return None;
    }
    let root_candidate = root.join(&rel);
    if !rel
        .components()
        .all(|part| matches!(part, std::path::Component::Normal(_)))
        || !root_candidate.is_file()
    {
        return None;
    }
    Some(SkillLinkFailure {
        code: "skill_local_reference_missing",
        detail: format!(
            "{skill_rel}: {} exists at package root but not from skill directory",
            rel.display()
        ),
    })
}

fn resolves_inside(root: &Path, path: &Path) -> bool {
    let Ok(root_abs) = root.canonicalize() else {
        return false;
    };
    let Ok(path_abs) = path.canonicalize() else {
        return false;
    };
    path_abs.strip_prefix(root_abs).is_ok() && path_abs.is_file()
}

fn normalized_ref(raw: &str) -> Option<PathBuf> {
    let trimmed = raw
        .trim()
        .trim_matches(|c| matches!(c, '<' | '>' | ',' | '.' | ';' | ':' | '\'' | '"'));
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with('/')
        || trimmed.contains("://")
        || trimmed.starts_with("mailto:")
    {
        return None;
    }
    let path = trimmed
        .split('#')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if [".md", ".json", ".toml", ".sh"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
    {
        Some(PathBuf::from(path))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn resolves_inside_returns_false_when_root_or_path_cannot_canonicalize() {
        assert!(!super::resolves_inside(
            Path::new("/definitely/missing/root"),
            Path::new("/definitely/missing/root/file.md")
        ));
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("skill-link-resolve");
        std::fs::create_dir_all(&root).expect("root");
        assert!(!super::resolves_inside(&root, &root.join("missing.md")));
        std::fs::remove_dir_all(root).expect("cleanup skill link resolve");
    }
}

fn extract_refs(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    refs.extend(backtick_refs(text));
    refs.extend(markdown_link_refs(text));
    refs
}

fn backtick_refs(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, value)| value.to_string())
        .collect()
}

fn markdown_link_refs(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        refs.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    refs
}
