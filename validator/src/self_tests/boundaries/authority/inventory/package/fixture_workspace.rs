use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn write_canonical_bin(root: &Path) {
    write_file(
        root,
        "validator/Cargo.toml",
        r#"
[[bin]]
name = "ultragoal"
path = "src/bin/ultragoal.rs"
"#,
    );
    write_file(root, "validator/src/bin/ultragoal.rs", "fn main() {}\n");
}

pub(super) fn write_manifest(root: &Path, paths: &[&str]) {
    let value = serde_json::json!({ "resources": paths });
    let path = root.join("plugin-manifest-draft.json");
    std::fs::write(path, serde_json::to_vec(&value).expect("manifest json")).expect("manifest");
}

pub(super) fn failures(root: &Path, inventory: BTreeSet<String>) -> Vec<(String, String)> {
    crate::audit::law::authority_surfaces::package_surface_failures_for_test(root, &inventory)
}

pub(super) fn inventory(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_string()).collect()
}

pub(super) fn temp_root(label: &str) -> std::path::PathBuf {
    crate::self_tests::boundaries::workspace_fixtures::temp_root(label)
}

pub(super) fn write_file(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(path, text).expect("write fixture file");
}

pub(super) fn assert_contains(failures: &[(String, String)], needle: &str) {
    assert!(failure_text(failures).contains(needle), "{failures:?}");
}

pub(super) fn failure_text(failures: &[(String, String)]) -> String {
    failures
        .iter()
        .map(|(_, failure)| failure.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn assert_row(
    inventory: &serde_json::Value,
    path_or_symbol: &str,
    kind: &str,
    authority: &str,
) {
    let rows = package_surface_rows(inventory)
        .and_then(serde_json::Value::as_array)
        .expect("active surface rows");
    assert!(
        rows.iter().any(|row| {
            row.get("path_or_symbol")
                .and_then(serde_json::Value::as_str)
                == Some(path_or_symbol)
                && row.get("surface_kind").and_then(serde_json::Value::as_str) == Some(kind)
                && row
                    .get("authority_level")
                    .and_then(serde_json::Value::as_str)
                    == Some(authority)
        }),
        "missing {authority} {kind} {path_or_symbol}: {inventory}"
    );
}

pub(super) fn assert_surface_id(
    inventory: &serde_json::Value,
    path_or_symbol: &str,
    surface_id: &str,
) {
    let rows = package_surface_rows(inventory)
        .and_then(serde_json::Value::as_array)
        .expect("package surface rows");
    assert!(
        rows.iter().any(|row| {
            row.get("path_or_symbol")
                .and_then(serde_json::Value::as_str)
                == Some(path_or_symbol)
                && row.get("surface_id").and_then(serde_json::Value::as_str) == Some(surface_id)
        }),
        "missing {surface_id} for {path_or_symbol}: {inventory}"
    );
}

fn package_surface_rows(inventory: &serde_json::Value) -> Option<&serde_json::Value> {
    inventory
        .pointer("/package_surface_inventory/rows")
        .or_else(|| inventory.get("rows"))
}

pub(super) fn cleanup(root: std::path::PathBuf) {
    std::fs::remove_dir_all(root).expect("cleanup package-surface fixture root");
}
