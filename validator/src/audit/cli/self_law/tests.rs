use super::{check, CliSelfLawCheckError, CliSelfLawCheckRequest};
use std::path::Path;

#[test]
fn composition_is_stable_fail_closed_and_zero_write() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-self-law-composition");
    for directory in ["fixtures/red", "schemas", "templates"] {
        std::fs::create_dir_all(root.join(directory)).expect("fixture directory");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        b"{\"name\":\"fixture\",\"resources\":[]}\n",
    )
    .expect("manifest");
    std::fs::write(root.join("templates/RED_FIXTURES.json"), b"[]\n").expect("red catalog");
    std::fs::write(
        root.join("schemas/schema-catalog.json"),
        b"{\"schemas\":[]}\n",
    )
    .expect("schema catalog");
    let before = tree(&root);
    let response = check(CliSelfLawCheckRequest::new(&root, 4)).expect("closed response");
    let findings = response.into_findings();
    assert!(!findings.is_empty());
    assert!(findings.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(findings
        .iter()
        .any(|finding| finding.check_id == "plugin-inventory-closure"));
    assert!(findings
        .iter()
        .any(|finding| finding.check_id == "cli-self-law-compliance"));
    assert_eq!(tree(&root), before);
    std::fs::remove_dir_all(root).expect("cleanup self law composition");
}

#[test]
fn zero_parallelism_is_a_closed_error() {
    let root = Path::new(".");
    let error = check(CliSelfLawCheckRequest::new(root, 0)).expect_err("zero jobs rejected");
    assert_eq!(error, CliSelfLawCheckError::InvalidParallelism);
    assert_eq!(error.id(), "cli_self_law_parallelism_invalid");
}

fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    rows
}

fn walk(root: &Path, directory: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
    let mut entries = std::fs::read_dir(directory)
        .expect("read fixture")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        let metadata = std::fs::symlink_metadata(&path).expect("metadata");
        if metadata.is_dir() {
            walk(root, &path, rows);
        } else if metadata.is_file() {
            rows.push((
                path.strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(path).expect("bytes"),
            ));
        }
    }
}
