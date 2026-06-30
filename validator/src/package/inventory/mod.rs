use crate::digest;
use crate::json_boundary;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub(crate) mod closure;
pub(crate) mod payload;

pub use closure::{final_bytecode_failures, inventory_closure_failures};
pub(crate) use payload::stable_package_payload;

pub const PACKAGE_DIGEST_EXCLUDED_PREFIXES: &[&str] = &["validation_artifacts/"];
pub const PACKAGE_DIGEST_EXCLUDED_PATHS: &[&str] = &[];
const PARENT_SESSION_CONTRACT_PREFIX: &str = "docs/parent-session-full-ultragoal-";

pub fn inventory_paths(manifest: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["skills", "agents"] {
        if let Some(rows) = manifest.get(key).and_then(Value::as_array) {
            for row in rows {
                if let Some(path) = row.get("path").and_then(Value::as_str) {
                    out.push(path.to_string());
                }
            }
        }
    }
    if let Some(path) = manifest.get("schema_catalog").and_then(Value::as_str) {
        out.push(path.to_string());
    }
    for key in [
        "schemas",
        "fixtures",
        "authorable_templates",
        "generated_examples",
        "resources",
    ] {
        if let Some(rows) = manifest.get(key).and_then(Value::as_array) {
            for row in rows {
                if let Some(path) = row.as_str() {
                    out.push(path.to_string());
                }
            }
        }
    }
    out
}

pub fn package_path_error(root: &Path, rel: &str) -> Option<String> {
    if rel.is_empty() {
        return Some("package path is not a non-empty string".to_string());
    }
    if parent_session_contract_path(rel) {
        return Some(format!(
            "parent-session contract is not a package resource: {rel}"
        ));
    }
    if Path::new(rel).is_absolute() {
        return Some(format!("package path is absolute: {rel}"));
    }
    if Path::new(rel)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Some(format!("package path escapes package root: {rel}"));
    }
    let root_abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if symlink_component_error(&root_abs, rel).is_some() {
        return Some(format!("package path uses symlink: {rel}"));
    }
    let full = root_abs.join(rel);
    let full = full.canonicalize().unwrap_or(full);
    canonical_escape_error(&root_abs, &full, rel)
}

pub(crate) fn parent_session_contract_path(rel: &str) -> bool {
    rel.starts_with(PARENT_SESSION_CONTRACT_PREFIX) && rel.ends_with(".md")
}

pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if let Some(error) = package_path_error(root, rel) {
        return Err(error);
    }
    Ok(root.join(Path::new(rel)))
}

fn symlink_component_error(root: &Path, rel: &str) -> Option<()> {
    let mut cursor = root.to_path_buf();
    for name in Path::new(rel).iter() {
        cursor.push(name);
        let Ok(meta) = std::fs::symlink_metadata(&cursor) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            return Some(());
        }
    }
    None
}

fn canonical_escape_error(root: &Path, full: &Path, rel: &str) -> Option<String> {
    if full.strip_prefix(root).is_err() {
        return Some(format!("package path escapes package root: {rel}"));
    }
    None
}

#[cfg(test)]
mod canonical_tests {
    use std::path::Path;

    #[test]
    fn canonical_escape_guard_reports_outside_package() {
        let root = Path::new("/package/root");
        assert!(super::canonical_escape_error(root, Path::new("/package/root/a"), "a").is_none());
        assert_eq!(
            super::canonical_escape_error(root, Path::new("/outside/a"), "a"),
            Some("package path escapes package root: a".to_string())
        );
    }
}

pub fn package_digest_excluded(rel: &str) -> bool {
    PACKAGE_DIGEST_EXCLUDED_PATHS.contains(&rel)
        || PACKAGE_DIGEST_EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| rel.starts_with(prefix))
}

pub fn package_digest(root: &Path) -> Result<String, String> {
    let manifest = json_boundary::read_json(&root.join("plugin-manifest-draft.json"))?;
    let mut paths = inventory_paths(&manifest)
        .into_iter()
        .filter(|rel| !package_digest_excluded(rel))
        .collect::<Vec<_>>();
    paths.sort();
    let mut payload = Vec::new();
    for rel in paths {
        payload.extend_from_slice(rel.as_bytes());
        payload.push(0);
        let bytes = match resolve(root, &rel) {
            Ok(path) if path.is_file() => crate::digest::read_file_bytes(&path)
                .map_err(|err| format!("{}: package digest read failed: {err}", path.display()))?,
            Ok(path) => {
                return Err(format!(
                    "{}: package digest manifest path is missing",
                    path.display()
                ));
            }
            Err(err) => return Err(format!("{rel}: package digest path invalid: {err}")),
        };
        payload.extend_from_slice(&payload::stable_package_payload(&rel, &bytes)?);
        payload.push(0);
    }
    Ok(digest::bytes(&payload))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    fn write_manifest(root: &std::path::Path, resources: serde_json::Value) {
        std::fs::write(
            root.join("plugin-manifest-draft.json"),
            serde_json::to_vec(&json!({"resources": resources})).expect("manifest"),
        )
        .expect("write manifest");
    }

    #[test]
    fn package_digest_rejects_missing_directories_and_invalid_paths() {
        let root = crate::self_tests::boundaries::support::temp_root("package-inventory-digest");
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::write(root.join("docs/file.txt"), "ok").expect("file");

        write_manifest(&root, json!(["docs/file.txt"]));
        assert!(
            super::package_digest(&root)
                .expect("valid digest")
                .starts_with("sha256:")
        );

        write_manifest(&root, json!(["docs"]));
        assert!(
            super::package_digest(&root)
                .expect_err("directory rejected")
                .contains("package digest manifest path is missing")
        );

        write_manifest(&root, json!(["docs/missing.txt"]));
        assert!(
            super::package_digest(&root)
                .expect_err("missing file rejected")
                .contains("package digest manifest path is missing")
        );

        write_manifest(&root, json!(["../escape.txt"]));
        assert!(
            super::package_digest(&root)
                .expect_err("escape rejected")
                .contains("package digest path invalid")
        );
        std::fs::remove_dir_all(root).expect("cleanup package inventory");
    }

    #[test]
    fn package_digest_excludes_mutable_final_packet_proof_receipt() {
        assert!(super::package_digest_excluded(
            "validation_artifacts/review/final-packet-proof.json"
        ));
        assert!(super::package_digest_excluded(
            "validation_artifacts/review/2026-06-25-session-log-hardening-packet.json"
        ));
        assert!(super::package_digest_excluded(
            "validation_artifacts/semantic-classification/minimal-goal-run/CLAIM-001.semantic-classification-receipt.json"
        ));
    }

    #[test]
    fn package_digest_rejects_parent_session_contract_resources() {
        let root = crate::self_tests::boundaries::support::temp_root("package-parent-contract");
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::write(root.join("docs/package.md"), "package resource").expect("package doc");
        std::fs::write(
            root.join("docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"),
            "builder contract",
        )
        .expect("parent prompt");
        write_manifest(&root, json!(["docs/package.md"]));
        let before = super::package_digest(&root).expect("digest before");
        std::fs::write(
            root.join("docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"),
            "updated builder contract",
        )
        .expect("parent prompt update");
        let after = super::package_digest(&root).expect("digest after");
        assert_eq!(before, after);

        write_manifest(
            &root,
            json!(["docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"]),
        );
        let err = super::package_digest(&root).expect_err("parent contract listed");
        assert!(
            err.contains("parent-session contract is not a package resource"),
            "{err}"
        );
        std::fs::remove_dir_all(root).expect("cleanup parent contract digest");
    }
}
