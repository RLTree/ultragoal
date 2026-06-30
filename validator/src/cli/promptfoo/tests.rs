use super::{PromptfooCommand, proof, registry};
use serde_json::json;
use std::path::{Path, PathBuf};

#[test]
fn promptfoo_receipt_blocks_raw_eval_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("promptfoo-receipt");
    seed_root(&root);
    let command = PromptfooCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        promptfoo_bin: fake_promptfoo(&root),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert!(proof::receipt_failures(&root, &receipt).is_empty());
    assert_eq!(
        receipt["raw_promptfoo_authority"],
        "observation_only_until_cli_parsed_receipt"
    );
    assert!(
        registry::array_strings(receipt.get("blocked_claims")).contains(&"update_goal_eligibility")
    );
}

#[test]
fn promptfoo_receipt_rejects_wrong_candidate() {
    let root = crate::self_tests::boundaries::support::temp_root("promptfoo-wrong-candidate");
    seed_root(&root);
    let command = PromptfooCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        promptfoo_bin: fake_promptfoo(&root),
    };
    let mut receipt = proof::build_receipt(&root, &command).expect("receipt");
    receipt["candidate_digest"] = json!(crate::digest::ZERO);
    assert!(
        proof::receipt_failures(&root, &receipt)
            .iter()
            .any(|item| item.contains("candidate_digest"))
    );
}

fn seed_root(root: &Path) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": [
            "package.json",
            "pnpm-lock.yaml",
            "pnpm-workspace.yaml",
            super::PROVIDER_REGISTRY,
            super::SUITE_REGISTRY
        ]}))
        .expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(
        root.join(super::PACKAGE_JSON),
        serde_json::to_vec(&json!({
            "packageManager": "pnpm@11.1.2",
            "devDependencies": {"promptfoo": super::PROMPTFOO_VERSION}
        }))
        .expect("package"),
    )
    .expect("package");
    std::fs::write(
        root.join(super::PNPM_LOCK),
        "promptfoo:\n  specifier: 0.121.17\n",
    )
    .expect("lock");
    std::fs::write(
        root.join(super::PNPM_WORKSPACE),
        "allowBuilds:\n  esbuild: false\n",
    )
    .expect("workspace");
    write_registries(root);
}

fn write_registries(root: &Path) {
    crate::json_boundary::write_json(
        &root.join(super::PROVIDER_REGISTRY),
        &json!({
            "schema": "harness-ultragoal.promptfoo-provider-registry.v1",
            "law_id": super::LAW_ID,
            "providers": [
                {"mode": "no_network"},
                {"mode": "offline_fixture"},
                {"mode": "local_mock"},
                {"mode": "openai_live"}
            ],
            "forbidden_substitutions": [
                "raw_promptfoo_pass_as_claim",
                "promptfoo_eval_as_product_success"
            ]
        }),
    )
    .expect("providers");
    crate::json_boundary::write_json(
        &root.join(super::SUITE_REGISTRY),
        &json!({
            "schema": "harness-ultragoal.promptfoo-suite-registry.v1",
            "law_id": super::LAW_ID,
            "suites": [{"id": "claim-ceiling-provider-separation-smoke"}]
        }),
    )
    .expect("suites");
}

#[cfg(unix)]
fn fake_promptfoo(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join("fake-promptfoo");
    std::fs::write(&path, "#!/bin/sh\nprintf '0.121.17\\n'\n").expect("fake");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("mode");
    path
}

#[cfg(not(unix))]
fn fake_promptfoo(_root: &Path) -> PathBuf {
    PathBuf::from("node_modules/.bin/promptfoo")
}
