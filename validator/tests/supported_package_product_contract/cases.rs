use super::product::{CANONICAL_SKILLS, PLUGIN_ID, SUPPORTED_MANIFEST_PATH, SUPPORTED_VERSION};
use super::*;
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{AuthorityCatalog, GeneratedSurfaceIndex};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct ProductRoot {
    root: PathBuf,
}

impl ProductRoot {
    fn new(label: &str) -> Self {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
        fs::create_dir_all(root.join(".codex-plugin")).expect("plugin manifest directory");
        fs::create_dir_all(root.join("schemas")).expect("schema directory");
        fs::write(root.join("schemas/catalog.json"), "{}\n").expect("schema catalog");
        fs::write(
            root.join(SUPPORTED_MANIFEST_PATH),
            serde_json::to_vec(&json!({
                "name": PLUGIN_ID,
                "version": SUPPORTED_VERSION,
                "description": "Supported package contract fixture.",
                "author": {"name": "Test"},
                "license": "UNLICENSED",
                "keywords": ["test"],
                "skills": "./skills/",
                "interface": {
                    "displayName": "Harness Ultragoal",
                    "shortDescription": "Contract fixture.",
                    "longDescription": "Supported package contract fixture.",
                    "developerName": "Test",
                    "category": "Productivity",
                    "capabilities": ["Read"]
                }
            }))
            .expect("supported manifest JSON"),
        )
        .expect("supported manifest");
        for name in CANONICAL_SKILLS {
            let skill_root = root.join("skills").join(name);
            fs::create_dir_all(skill_root.join("agents")).expect("skill metadata directory");
            fs::write(
                skill_root.join("SKILL.md"),
                format!("---\nname: {name}\n---\n"),
            )
            .expect("skill");
            fs::write(
                skill_root.join("agents/openai.yaml"),
                format!("interface:\n  display_name: {name}\n"),
            )
            .expect("skill metadata");
        }
        write_draft(&root, SUPPORTED_VERSION);
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&root)
                .status()
                .expect("git init")
                .success()
        );
        Self { root }
    }

    fn context(&self) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_repository_root(&self.root)
                .expect_worktree_root(&self.root),
        )
        .expect("live context")
    }
}

impl Drop for ProductRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_draft(root: &Path, version: &str) {
    let skills = CANONICAL_SKILLS
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "path": format!("skills/{name}/SKILL.md"),
                "role": "test"
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "agents": [],
            "authorable_templates": [],
            "fixtures": [],
            "generated_examples": [],
            "name": PLUGIN_ID,
            "non_goals": [],
            "optional_connectors": [],
            "purpose": "test",
            "resources": ["plugin-manifest-draft.json"],
            "schema_catalog": "schemas/catalog.json",
            "schemas": [],
            "skills": skills,
            "status": "test",
            "version": version
        }))
        .expect("draft manifest JSON"),
    )
    .expect("draft manifest");
}

fn catalog(context: &LiveContext) -> AuthorityCatalog {
    AuthorityCatalog::canonical_for_test(
        context.context_id().to_owned(),
        "test-contract".to_owned(),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("canonical test catalog")
}

#[test]
fn named_target_exercises_real_product_capture_and_catalog_binding() {
    let product = ProductRoot::new("named-positive");
    let context = product.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("capture");
    verify_product_package(&artifact, &context, &authority_catalog).expect("verify");
    assert_eq!(artifact.snapshot().entries().len(), 17);
}
