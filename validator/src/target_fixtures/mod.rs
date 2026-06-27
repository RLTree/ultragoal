mod baseline_specs;
mod observability_specs;
mod product_cohesion_specs;
mod required_files;
mod spec;

use crate::digest;
use crate::json_boundary;
use crate::target_repo;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

pub(crate) use spec::model::TargetSpec;

pub fn target_capability_failures(root: &Path, artifacts: &[Value]) -> Vec<String> {
    target_capability_failures_for(root, artifacts, &spec::catalog::specs())
}

pub(crate) fn target_capability_failures_for(
    root: &Path,
    artifacts: &[Value],
    specs: &[TargetSpec],
) -> Vec<String> {
    let mut failures = required_file_failures(root);
    for item in specs {
        let target = root.join(item.rel);
        if !target.exists() {
            failures.push(format!("missing target repo fixture: {}", item.rel));
            continue;
        }
        let (receipt, code) = audit_spec(root, &item, artifacts);
        if code != item.expected_code {
            failures.push(format!(
                "{} expected exit {}, got {code}: {}",
                item.rel, item.expected_code, receipt["status"]
            ));
        }
        if let (Some(check), Some(status)) = (item.expected_check, item.expected_status)
            && receipt
                .pointer(&format!("/checks/{}/status", json_pointer_token(check)))
                .and_then(Value::as_str)
                != Some(status)
        {
            failures.push(format!("{} expected {check}={status}", item.rel));
        }
    }
    failures
}

pub fn write_target_receipts(
    root: &Path,
    out_dir: &Path,
    artifacts: &[Value],
) -> Result<Vec<Value>, String> {
    let mut generated = Vec::new();
    for item in spec::catalog::specs() {
        let (receipt, _) = audit_spec(root, &item, artifacts);
        let path = out_dir.join(item.name);
        json_boundary::write_json(&path, &receipt)?;
        generated.push(json!({
            "artifact_type": "target_repo_receipt",
            "path": path.to_string_lossy(),
            "digest": digest::file(&path)?,
            "validator_run_id": "pending",
            "input_digest": digest::file(&root.join("schemas/target-repo-receipt.schema.json"))?,
            "generated_at": crate::audit::clock::now_iso()
        }));
    }
    Ok(generated)
}

fn audit_spec(root: &Path, item: &spec::model::TargetSpec, artifacts: &[Value]) -> (Value, i32) {
    let target = root.join(item.rel);
    let symlink_fixture = match materialize_symlink_fixture(&target) {
        Ok(fixture) => fixture,
        Err(err) => {
            return (
                json!({"schema": "harness-ultragoal.target-repo-receipt.v1", "status": "fail", "checks": {}, "error": err}),
                1,
            );
        }
    };
    let result = target_repo::audit_target_repo(
        &target,
        item.mode,
        &format!("fixture {} {}", item.mode, item.rel),
        artifacts,
        item.require_observability,
        item.require_product,
        Some(root),
    );
    result_after_cleanup(symlink_fixture, result)
}

#[derive(Debug)]
pub(crate) struct MaterializedSymlink {
    pub(crate) link: PathBuf,
    pub(crate) target_rel: PathBuf,
}

impl MaterializedSymlink {
    pub(crate) fn cleanup(&self) -> Result<(), String> {
        if !self.link.exists() && !self.link.is_symlink() {
            return Ok(());
        }
        let observed = read_link_result(
            std::fs::read_link(&self.link),
            "symlink fixture readlink cleanup failed",
        )?;
        if observed != self.target_rel {
            return Err("symlink fixture cleanup refused changed target".to_string());
        }
        remove_result(
            std::fs::remove_file(&self.link),
            "symlink fixture cleanup failed",
        )
    }
}

pub(crate) fn result_after_cleanup(
    symlink_fixture: Option<MaterializedSymlink>,
    result: (Value, i32),
) -> (Value, i32) {
    if let Some(fixture) = symlink_fixture
        && let Err(err) = fixture.cleanup()
    {
        return (
            json!({"schema": "harness-ultragoal.target-repo-receipt.v1", "status": "fail", "checks": {}, "error": err}),
            1,
        );
    }
    result
}

pub(crate) fn materialize_symlink_fixture(
    target: &Path,
) -> Result<Option<MaterializedSymlink>, String> {
    let meta_path = target.join("validation_artifacts/product-cohesion/symlink-fixture.json");
    if !meta_path.is_file() {
        return Ok(None);
    }
    let meta = json_boundary::read_json(&meta_path)?;
    if meta.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.target-fixture-symlink.v1")
    {
        return Err("target symlink fixture schema mismatch".to_string());
    }
    let link_rel = meta.get("link_path").and_then(Value::as_str).unwrap_or("");
    let target_rel = meta
        .get("target_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !normal_relative(link_rel) || target_rel.is_empty() || Path::new(target_rel).is_absolute() {
        return Err("target symlink fixture path invalid".to_string());
    }
    let link = target.join(link_rel);
    let target_rel = PathBuf::from(target_rel);
    if link.is_symlink() {
        let observed =
            read_link_result(std::fs::read_link(&link), "symlink fixture readlink failed")?;
        if observed != target_rel {
            return Err("target symlink fixture link target mismatch".to_string());
        }
        return Ok(Some(MaterializedSymlink { link, target_rel }));
    }
    if link.exists() {
        return Err("target symlink fixture link path already exists".to_string());
    }
    let parent = link.parent().expect("target fixture link path has parent");
    create_symlink_parent(parent)?;
    symlink_result(
        std::os::unix::fs::symlink(&target_rel, &link),
        "symlink fixture materialize failed",
    )?;
    Ok(Some(MaterializedSymlink { link, target_rel }))
}

pub(crate) fn create_symlink_parent(parent: &Path) -> Result<(), String> {
    std::fs::create_dir_all(parent).map_err(|err| format!("symlink fixture mkdir: {err}"))
}

fn read_link_result(result: std::io::Result<PathBuf>, label: &str) -> Result<PathBuf, String> {
    result.map_err(|err| format!("{label}: {err}"))
}

fn remove_result(result: std::io::Result<()>, label: &str) -> Result<(), String> {
    result.map_err(|err| format!("{label}: {err}"))
}

fn symlink_result(result: std::io::Result<()>, label: &str) -> Result<(), String> {
    result.map_err(|err| format!("{label}: {err}"))
}

pub(crate) fn normal_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn json_pointer_token(raw: &str) -> String {
    raw.replace('~', "~0").replace('/', "~1")
}

fn required_file_failures(root: &Path) -> Vec<String> {
    required_files::REQUIRED_FILES
        .iter()
        .filter(|rel| !root.join(rel).is_file())
        .map(|rel| format!("missing target-repo audit files: {rel}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{create_symlink_parent, read_link_result, remove_result, symlink_result};
    use std::io::Error;

    #[test]
    fn symlink_fixture_os_error_mappers_are_testable() {
        assert!(
            read_link_result(
                Err(Error::other("denied")),
                "symlink fixture readlink failed"
            )
            .expect_err("readlink failure")
            .contains("readlink failed")
        );
        assert!(
            remove_result(Err(Error::other("busy")), "symlink fixture cleanup failed")
                .expect_err("remove failure")
                .contains("cleanup failed")
        );
        assert!(
            symlink_result(
                Err(Error::other("blocked")),
                "symlink fixture materialize failed"
            )
            .expect_err("symlink failure")
            .contains("materialize failed")
        );
        let tmp =
            std::env::temp_dir().join(format!("ultragoal-symlink-parent-{}", std::process::id()));
        create_symlink_parent(&tmp.join("nested")).expect("create symlink parent");
        std::fs::remove_dir_all(tmp).expect("cleanup symlink parent");
    }
}
