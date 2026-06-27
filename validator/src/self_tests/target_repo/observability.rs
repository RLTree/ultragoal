use serde_json::Value;
use std::path::{Path, PathBuf};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        let ty = entry.file_type().expect("file type");
        let dest = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy fixture file");
        }
    }
}

fn copied_fixture(label: &str, fixture: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::repo_root();
    let target = crate::self_tests::boundaries::support::temp_root(label);
    copy_dir(&root.join(fixture), &target);
    target
}

fn audit(root: &Path, required: bool) -> Value {
    let (receipt, _) = crate::target_repo::audit_target_repo(
        root,
        "fresh-init",
        "target observability test",
        &[],
        required,
        false,
        None,
    );
    receipt["checks"]["observability-stack"].clone()
}

fn detail(row: &Value) -> &str {
    row["detail"].as_str().expect("detail")
}

fn remove_marker(root: &Path) {
    let script = root.join("scripts/check");
    let text = std::fs::read_to_string(&script).expect("script");
    std::fs::write(
        &script,
        text.replace("echo \"harness-check:observability pass\"\n", ""),
    )
    .expect("remove marker");
}

#[test]
fn observability_audit_rejects_missing_marker_and_bad_event_surfaces() {
    let no_marker = copied_fixture(
        "observability-no-marker",
        "fixtures/target-repo/valid-observability",
    );
    remove_marker(&no_marker);
    let marker_row = audit(&no_marker, false);
    assert_eq!(marker_row["status"], "fail");
    assert!(detail(&marker_row).contains("observability marker missing"));
    std::fs::remove_dir_all(no_marker).expect("cleanup marker");

    let bad_utf8 = copied_fixture(
        "observability-bad-utf8",
        "fixtures/target-repo/valid-observability",
    );
    std::fs::write(
        bad_utf8.join("validation_artifacts/observability/events.jsonl"),
        [0xff, 0xfe],
    )
    .expect("bad events");
    let utf8_row = audit(&bad_utf8, true);
    assert!(detail(&utf8_row).contains("events.jsonl unreadable"));
    std::fs::remove_dir_all(bad_utf8).expect("cleanup bad utf8");

    let empty_rows = copied_fixture(
        "observability-empty-rows",
        "fixtures/target-repo/valid-observability",
    );
    std::fs::write(
        empty_rows.join("validation_artifacts/observability/events.jsonl"),
        " \n\n",
    )
    .expect("empty events");
    let empty_row = audit(&empty_rows, true);
    assert!(detail(&empty_row).contains("lacks a complete event row"));
    std::fs::remove_dir_all(empty_rows).expect("cleanup empty rows");
}

#[test]
fn observability_query_command_rejects_unreadable_empty_and_side_effect_scripts() {
    let unreadable = copied_fixture(
        "observability-unreadable-script",
        "fixtures/target-repo/valid-observability",
    );
    std::fs::write(unreadable.join("scripts/observe"), [0xff, 0xfe]).expect("bad script");
    let unreadable_row = audit(&unreadable, true);
    assert!(detail(&unreadable_row).contains("query command unreadable"));
    std::fs::remove_dir_all(unreadable).expect("cleanup unreadable");

    let empty = copied_fixture(
        "observability-empty-script",
        "fixtures/target-repo/valid-observability",
    );
    std::fs::write(empty.join("scripts/observe"), "").expect("empty script");
    let empty_row = audit(&empty, true);
    assert!(detail(&empty_row).contains("query command empty"));
    std::fs::remove_dir_all(empty).expect("cleanup empty");

    let unsafe_script = copied_fixture(
        "observability-unsafe-script",
        "fixtures/target-repo/valid-observability",
    );
    std::fs::write(
        unsafe_script.join("scripts/observe"),
        "#!/usr/bin/env bash\nrm -rf validation_artifacts\n",
    )
    .expect("unsafe script");
    let unsafe_row = audit(&unsafe_script, true);
    assert!(detail(&unsafe_row).contains("not static/read-only"));
    std::fs::remove_dir_all(unsafe_script).expect("cleanup unsafe");
}
