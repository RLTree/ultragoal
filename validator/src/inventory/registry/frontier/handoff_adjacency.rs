use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use crate::package::inventory::{inventory_paths, package_digest_excluded};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const RECEIPT: &str = "validation_artifacts/worker-results/N11-EVAL-RESEARCH.json";

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    record: &Value,
) -> Result<(), InventoryError> {
    if record.get("lane_id").and_then(Value::as_str) != Some("N11") {
        return unissued(record);
    }
    match record.get("status").and_then(Value::as_str) {
        Some("issued") => unissued(record),
        Some("ready") => verified(reads, root, record),
        _ => Err(invalid("lease handoff has an illegal status")),
    }
}

fn unissued(record: &Value) -> Result<(), InventoryError> {
    if record
        .pointer("/receipt_child/status")
        .and_then(Value::as_str)
        == Some("unissued")
    {
        Ok(())
    } else {
        Err(invalid(
            "lease receipt child precedes a verified source handoff",
        ))
    }
}

fn verified(reads: &ReadSession, root: &Path, record: &Value) -> Result<(), InventoryError> {
    verify(root, record)?;
    reads
        .revalidate()
        .map_err(|_| invalid("lease handoff validation became stale"))
}

fn verify(root: &Path, record: &Value) -> Result<(), InventoryError> {
    let source = handoff(record)?;
    let base = text(record.get("base_commit"), "lease source base is missing")?;
    if git(root, &["rev-parse", &format!("{source}^{{tree}}")])?
        != text(
            record.pointer("/handoff/tree"),
            "lease handoff tree is missing",
        )?
        || git(root, &["rev-parse", &format!("{source}^")])? != base
    {
        return Err(invalid("lease source handoff is not adjacent to its base"));
    }
    let child = record
        .get("receipt_child")
        .ok_or_else(|| invalid("lease receipt child is missing"))?;
    if child.get("status").and_then(Value::as_str) != Some("verified")
        || child.get("path").and_then(Value::as_str) != Some(RECEIPT)
        || text(
            child.get("source_commit"),
            "lease receipt source commit is missing",
        )? != source
        || text(
            child.get("source_tree"),
            "lease receipt source tree is missing",
        )? != text(
            record.pointer("/handoff/tree"),
            "lease handoff tree is missing",
        )?
    {
        return Err(invalid(
            "lease receipt child is not bound to its source handoff",
        ));
    }
    let child_commit = text(child.get("commit"), "lease receipt child commit is missing")?;
    if git(root, &["rev-parse", &format!("{child_commit}^{{tree}}")])?
        != text(child.get("tree"), "lease receipt child tree is missing")?
        || git(root, &["rev-parse", &format!("{child_commit}^")])?
            != text(
                child.get("parent_commit"),
                "lease receipt parent commit is missing",
            )?
        || text(
            child.get("parent_commit"),
            "lease receipt parent commit is missing",
        )? != source
        || git(root, &["rev-parse", &format!("{source}^{{tree}}")])?
            != text(
                child.get("parent_tree"),
                "lease receipt parent tree is missing",
            )?
    {
        return Err(invalid(
            "lease receipt child is not adjacent to source handoff",
        ));
    }
    let child_paths = git(
        root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            &format!("{child_commit}^"),
            child_commit,
        ],
    )?;
    if child_paths.lines().collect::<Vec<_>>() != [RECEIPT]
        || !git_exists(root, &format!("{child_commit}:{RECEIPT}"))?
    {
        return Err(invalid(
            "lease receipt child does not contain exactly one receipt path",
        ));
    }
    let source_paths = git(root, &["diff", "--name-only", "--no-renames", base, source])?;
    if source_paths.lines().any(|path| path == RECEIPT)
        || git_exists(root, &format!("{source}:{RECEIPT}"))?
    {
        return Err(invalid("lease source handoff contains its receipt child"));
    }
    package_excluded(root, source)?;
    if record.get("ready_receipt").and_then(Value::as_str) != Some(RECEIPT) {
        return Err(invalid(
            "lease ready receipt does not name its sole receipt child",
        ));
    }
    Ok(())
}

fn package_excluded(root: &Path, source: &str) -> Result<(), InventoryError> {
    if !package_digest_excluded(RECEIPT) {
        return Err(invalid(
            "lease receipt child is not excluded from package authority",
        ));
    }
    let bytes = git(
        root,
        &["show", &format!("{source}:plugin-manifest-draft.json")],
    )?;
    let manifest: Value = serde_json::from_str(&bytes)
        .map_err(|_| invalid("lease source package manifest is malformed"))?;
    if inventory_paths(&manifest)
        .iter()
        .any(|path| path == RECEIPT)
    {
        return Err(invalid("lease receipt child is package-visible"));
    }
    Ok(())
}

fn handoff(record: &Value) -> Result<&str, InventoryError> {
    if record.pointer("/handoff/status").and_then(Value::as_str) != Some("verified") {
        return Err(invalid("lease handoff is not verified"));
    }
    text(
        record.pointer("/handoff/commit"),
        "lease handoff commit is missing",
    )
}

fn git(root: &Path, args: &[&str]) -> Result<String, InventoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|_| invalid("cannot inspect lease handoff Git identity"))?;
    if !output.status.success() {
        return Err(invalid("cannot inspect lease handoff Git identity"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid("lease handoff Git identity is not UTF-8"))
}
fn git_exists(root: &Path, spec: &str) -> Result<bool, InventoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "-e", spec])
        .output()
        .map_err(|_| invalid("cannot inspect lease receipt child"))?;
    Ok(output.status.success())
}
fn text<'a>(value: Option<&'a Value>, message: &str) -> Result<&'a str, InventoryError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{RECEIPT, verify};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn receipt_child_requires_exact_adjacency_and_package_exclusion() {
        let root = fixture();
        let base = git(&root, &["rev-parse", "HEAD"]);
        fs::create_dir_all(root.join("validator/src/evaluation")).unwrap();
        fs::write(root.join("validator/src/evaluation/journal.rs"), "source").unwrap();
        commit(&root, "source");
        let source = git(&root, &["rev-parse", "HEAD"]);
        let source_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
        fs::create_dir_all(root.join("validation_artifacts/worker-results")).unwrap();
        fs::write(root.join(RECEIPT), "receipt").unwrap();
        commit(&root, "receipt");
        let child = git(&root, &["rev-parse", "HEAD"]);
        let child_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
        let record = json!({"status":"ready","base_commit":base,"handoff":{"status":"verified","commit":source,"tree":source_tree},"receipt_child":{"status":"verified","path":RECEIPT,"commit":child,"tree":child_tree,"parent_commit":source,"parent_tree":source_tree,"source_commit":source,"source_tree":source_tree},"ready_receipt":RECEIPT});
        verify(&root, &record).unwrap();
        let mut forged = record;
        forged["receipt_child"]["parent_commit"] = forged["base_commit"].clone();
        assert!(
            verify(&root, &forged)
                .unwrap_err()
                .to_string()
                .contains("not adjacent")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inherited_receipt_path_is_not_a_valid_source_handoff() {
        let root = fixture();
        fs::create_dir_all(root.join("validation_artifacts/worker-results")).unwrap();
        fs::write(root.join(RECEIPT), "old").unwrap();
        commit(&root, "old receipt");
        let base = git(&root, &["rev-parse", "HEAD"]);
        fs::create_dir_all(root.join("validator/src/evaluation")).unwrap();
        fs::write(root.join("validator/src/evaluation/journal.rs"), "source").unwrap();
        commit(&root, "source");
        let source = git(&root, &["rev-parse", "HEAD"]);
        let source_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
        fs::write(root.join(RECEIPT), "new").unwrap();
        commit(&root, "receipt");
        let child = git(&root, &["rev-parse", "HEAD"]);
        let child_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
        let record = json!({"status":"ready","base_commit":base,"handoff":{"status":"verified","commit":source,"tree":source_tree},"receipt_child":{"status":"verified","path":RECEIPT,"commit":child,"tree":child_tree,"parent_commit":source,"parent_tree":source_tree,"source_commit":source,"source_tree":source_tree},"ready_receipt":RECEIPT});
        assert!(
            verify(&root, &record)
                .unwrap_err()
                .to_string()
                .contains("contains its receipt child")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-handoff-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        for args in [
            ["init", "-q"].as_slice(),
            ["config", "user.email", "test@example.invalid"].as_slice(),
            ["config", "user.name", "Test"].as_slice(),
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        fs::write(
            root.join("plugin-manifest-draft.json"),
            "{\"resources\":[]}",
        )
        .unwrap();
        commit(&root, "base");
        root
    }
    fn commit(root: &PathBuf, message: &str) {
        assert!(
            Command::new("git")
                .args(["add", "-A"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args(["commit", "-q", "-m", message])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
    fn git(root: &PathBuf, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
