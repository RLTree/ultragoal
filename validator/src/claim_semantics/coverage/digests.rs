use serde_json::Value;
use std::path::Path;

pub(crate) fn source_tree_digest(root: &Path, manifest: &Value) -> Result<String, String> {
    let mut files = Vec::new();
    for target in strings(manifest, "required_target_paths") {
        let base = crate::package::inventory::resolve(root, &target)?;
        if base.is_file() {
            files.push(target);
            continue;
        }
        for entry in walkdir::WalkDir::new(&base).into_iter().flatten() {
            if entry.file_type().is_file() {
                let rel = source_rel_path(root, entry.path())?;
                if !ignored(&rel, manifest) {
                    files.push(rel);
                }
            }
        }
    }
    files.sort();
    files.dedup();
    digest_files(root, &files)
}

pub(crate) fn changed_files_digest(root: &Path, manifest: &Value) -> Result<String, String> {
    let mut files = manifest
        .pointer("/changed_file_coupling_policy/changed_files")
        .and_then(Value::as_array)
        .ok_or_else(|| "changed_files missing from coverage manifest".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    digest_files(root, &files)
}

fn digest_files(root: &Path, files: &[String]) -> Result<String, String> {
    let mut payload = Vec::new();
    for rel in files {
        payload.extend_from_slice(rel.as_bytes());
        payload.push(0);
        let bytes = digest_file_bytes(rel, crate::digest::read_file_bytes(&root.join(rel)))?;
        payload.extend_from_slice(&bytes);
        payload.push(0);
    }
    Ok(crate::digest::bytes(&payload))
}

fn ignored(rel: &str, manifest: &Value) -> bool {
    if crate::package::inventory::parent_session_contract_path(rel) {
        return true;
    }
    if matches!(
        rel,
        "templates/.harness/coverage-manifest.json"
            | "templates/.harness/coverage-command"
            | ".harness/coverage-manifest.json"
            | ".harness/coverage-command"
    ) {
        return true;
    }
    strings(
        manifest
            .get("source_discovery_rules")
            .unwrap_or(&Value::Null),
        "ignore",
    )
    .iter()
    .any(|pattern| ignore_match(rel, pattern))
}

fn ignore_match(rel: &str, pattern: &str) -> bool {
    if rel == pattern {
        return true;
    }
    pattern
        .strip_suffix("/**")
        .is_some_and(|prefix| rel == prefix || rel.starts_with(&format!("{prefix}/")))
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn source_rel_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|err| format!("source tree strip failed: {err}"))
}

pub(crate) fn digest_file_bytes(
    rel: &str,
    result: Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    result.map_err(|err| format!("{rel}: coverage digest read failed: {err}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn changed_files_digest_matches_canonical_sorted_unique_script_order() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("ultragoal-coverage-changed-files-digest-{stamp}"));
        std::fs::create_dir_all(root.join("validator/src")).expect("source dir");
        std::fs::write(root.join("validator/src/a.rs"), "a").expect("a");
        std::fs::write(root.join("validator/src/b.rs"), "b").expect("b");
        let manifest = json!({
            "changed_file_coupling_policy": {
                "changed_files": [
                    "validator/src/b.rs",
                    "validator/src/a.rs",
                    "validator/src/b.rs"
                ]
            }
        });
        let actual = super::changed_files_digest(&root, &manifest).expect("digest");
        let expected = super::digest_files(
            &root,
            &[
                "validator/src/a.rs".to_string(),
                "validator/src/b.rs".to_string(),
            ],
        )
        .expect("expected digest");
        assert_eq!(actual, expected);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
