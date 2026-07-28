use super::{
    INPUT, OUTPUT, assert_rejected_without_secret, digest, retained, root, source_projection,
    write_manifest, write_registry,
};
use serde_json::json;
use std::fs;

#[test]
fn shared_package_session_rejects_generated_cross_resource_mutation() {
    let root = root("generated-cross-resource-mutation");
    fs::write(root.join(INPUT), b"input").expect("input");
    let input_row = json!({"path": INPUT, "sha256": digest(b"input")});
    let output = serde_json::to_vec(&json!({
        "_meta": {"generator": "HCT-INVENTORY", "inputs": [input_row.clone()], "recipe": "input-digest-index-v1"},
        "entries": [input_row]
    }))
    .expect("output bytes");
    fs::write(root.join(OUTPUT), output).expect("output");
    write_registry(&root, source_projection(&root));
    let input = root.join(INPUT);
    super::super::super::anchored::test_hooks::set_before_component(OUTPUT, 0, move || {
        fs::write(&input, super::SECRET).expect("mutate generated input");
    });
    assert_rejected_without_secret(
        &super::Catalog::load(&root)
            .err()
            .expect("mixed generated snapshot"),
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn generated_disposition_errors_do_not_echo_untrusted_safe_paths() {
    const CANARY_PATH: &str = "docs/generated/SECRET_CANARY.json";
    let root = root("generated-untrusted-path-non-echo");
    fs::write(root.join(CANARY_PATH), b"context").expect("generated context");
    fs::write(root.join(OUTPUT), b"context").expect("registered context");
    write_registry(&root, retained(b"context"));
    write_manifest(&root, &[CANARY_PATH]);
    let error = super::super::super::package_digest(&root).expect_err("missing disposition");
    assert!(error.contains("no adopted disposition"), "{error}");
    assert!(!error.contains("SECRET_CANARY"), "{error}");
    fs::remove_dir_all(root).expect("cleanup");
}
