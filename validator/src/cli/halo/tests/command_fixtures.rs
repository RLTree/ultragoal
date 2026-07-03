use serde_json::json;
use std::path::{Path, PathBuf};

pub(super) fn seed_root(root: &Path) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": ["docs/halo-adapter-registry.json"]}))
            .expect("manifest"),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join(super::super::REGISTRY),
        &json!({
            "schema": "harness-ultragoal.halo-adapter-registry.v1",
            "law_id": super::super::LAW_ID,
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

pub(super) fn root_without_registry() -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("halo-bad-registry");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    crate::json_boundary::write_json(&root.join(super::super::REGISTRY), &json!({}))
        .expect("registry");
    root
}

pub(super) fn fake_app(root: &Path, version: &str) -> PathBuf {
    fake_app_with_identifier(root, "net.inference.halo", version)
}

pub(super) fn fake_app_with_identifier(root: &Path, identifier: &str, version: &str) -> PathBuf {
    let app = root.join("HALO.app");
    let contents = app.join("Contents");
    std::fs::create_dir_all(&contents).expect("contents");
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            "<plist><dict><key>CFBundleIdentifier</key><string>{identifier}</string><key>CFBundleVersion</key><string>{version}</string></dict></plist>"
        ),
    )
    .expect("plist");
    app
}
