use crate::audit::contract::Failure;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn check(root: &Path, ready: &Value, out: &mut Vec<Failure>) {
    let ready_files = strings(ready, "changed_files");
    if ready_files.is_empty() {
        return;
    }
    let path = if root.join(".harness/coverage-manifest.json").is_file() {
        root.join(".harness/coverage-manifest.json")
    } else {
        root.join("templates/.harness/coverage-manifest.json")
    };
    let Ok(manifest) = crate::json_boundary::read_json(&path) else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_changed_file_without_receipt",
            "coverage-manifest",
        ));
        return;
    };
    let measured = manifest
        .pointer("/changed_file_coupling_policy/changed_files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    for rel in ready_files {
        if !measured.contains(rel.as_str()) {
            out.push(Failure::new(
                "coverage-proof-policy",
                "coverage_changed_file_not_measured",
                rel,
            ));
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::audit::contract::Failure;
    use serde_json::{Value, json};
    use std::path::Path;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
    }

    #[test]
    fn coverage_ready_join_reports_missing_fallback_and_unmeasured_changes() {
        let root = crate::self_tests::boundaries::support::temp_root("coverage-ready-join");
        let ready = json!({"changed_files":["src/lib.rs"]});
        let mut out = Vec::<Failure>::new();
        super::check(&root, &ready, &mut out);
        assert!(
            out.iter()
                .any(|failure| failure.error == "coverage_changed_file_without_receipt")
        );

        write_json(
            &root.join("templates/.harness/coverage-manifest.json"),
            &json!({"changed_file_coupling_policy":{"changed_files":["src/other.rs"]}}),
        );
        out.clear();
        super::check(&root, &ready, &mut out);
        assert!(out.iter().any(|failure| failure.detail == "src/lib.rs"));

        write_json(
            &root.join(".harness/coverage-manifest.json"),
            &json!({"changed_file_coupling_policy":{"changed_files":["src/lib.rs"]}}),
        );
        out.clear();
        super::check(&root, &ready, &mut out);
        assert!(out.is_empty(), "{out:?}");

        out.clear();
        super::check(&root, &json!({"changed_files":[]}), &mut out);
        assert!(out.is_empty(), "{out:?}");
        std::fs::remove_dir_all(root).expect("cleanup coverage ready join");
    }
}
