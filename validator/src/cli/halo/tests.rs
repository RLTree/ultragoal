use super::{HaloCommand, proof};
use serde_json::json;
use std::path::{Path, PathBuf};

#[test]
fn halo_desktop_capability_is_manual_only() {
    let root = crate::self_tests::boundaries::support::temp_root("halo-capability");
    seed_root(&root);
    let app = fake_app(&root, "0.1.17");
    let command = HaloCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        app_path: app,
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["invocation_mode"], "desktop_manual");
    assert_eq!(receipt["authority_class"], "manual_observation_only");
    assert!(proof::receipt_failures(&root, &receipt).is_empty());
}

#[test]
fn halo_capability_rejects_overbroad_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("halo-overbroad");
    seed_root(&root);
    let command = HaloCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        app_path: fake_app(&root, "0.1.17"),
    };
    let mut receipt = proof::build_receipt(&root, &command).expect("receipt");
    receipt["authority_class"] = json!("adapter_required_before_claims");
    assert!(
        proof::receipt_failures(&root, &receipt)
            .iter()
            .any(|item| item == "halo_capability_authority_overbroad")
    );
}

fn seed_root(root: &Path) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": ["docs/halo-adapter-registry.json"]}))
            .expect("manifest"),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join(super::REGISTRY),
        &json!({
            "schema": "harness-ultragoal.halo-adapter-registry.v1",
            "law_id": super::LAW_ID,
            "modes": [
                {"id": "desktop_manual"},
                {"id": "cli"},
                {"id": "api"},
                {"id": "fixture"},
                {"id": "unavailable"}
            ],
            "forbidden_substitutions": [
                "halo_manual_output_as_deterministic_authority",
                "halo_desktop_presence_as_ranked_change_proof",
                "halo_recommendation_as_readiness",
                "halo_recommendation_as_update_goal",
                "halo_ranking_without_codex_handoff",
                "halo_ranking_without_validation_closure"
            ]
        }),
    )
    .expect("registry");
}

fn fake_app(root: &Path, version: &str) -> PathBuf {
    let app = root.join("HALO.app");
    let contents = app.join("Contents");
    std::fs::create_dir_all(&contents).expect("contents");
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            "<plist><dict><key>CFBundleIdentifier</key><string>net.inference.halo</string><key>CFBundleVersion</key><string>{version}</string></dict></plist>"
        ),
    )
    .expect("plist");
    app
}
