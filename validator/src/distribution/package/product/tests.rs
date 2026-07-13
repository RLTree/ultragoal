use super::*;
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{AuthorityCatalog, GeneratedSurfaceIndex};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Repo {
    root: PathBuf,
}

impl Repo {
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
                "description": "Supported product package test fixture.",
                "author": {"name": "Test"},
                "license": "UNLICENSED",
                "keywords": ["test"],
                "skills": "./skills/",
                "interface": {
                    "displayName": "Harness Ultragoal",
                    "shortDescription": "Test fixture.",
                    "longDescription": "Supported product package test fixture.",
                    "developerName": "Test",
                    "category": "Productivity",
                    "capabilities": ["Read"]
                }
            }))
            .expect("supported manifest JSON"),
        )
        .expect("supported manifest");
        for name in CANONICAL_SKILLS {
            let root = root.join("skills").join(name);
            fs::create_dir_all(root.join("agents")).expect("skill metadata directory");
            fs::write(root.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("skill");
            fs::write(
                root.join("agents/openai.yaml"),
                format!("interface:\n  display_name: {name}\n"),
            )
            .expect("skill metadata");
        }
        write_draft(&root, SUPPORTED_VERSION);
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .status()
            .expect("git init");
        assert!(status.success());
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

impl Drop for Repo {
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
        .expect("draft JSON"),
    )
    .expect("draft manifest");
}

fn catalog(context: &LiveContext) -> AuthorityCatalog {
    AuthorityCatalog::new(
        format!("sha256:{}", "1".repeat(64)),
        context.context_id().to_owned(),
        "test-contract".to_owned(),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        GeneratedSurfaceIndex::new(Vec::new()),
    )
}

fn status(root: &Path) -> Vec<u8> {
    Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .current_dir(root)
        .output()
        .expect("git status")
        .stdout
}

#[test]
fn independent_product_captures_are_byte_identical_and_reverified() {
    let repo = Repo::new("supported-package-product-identical");
    let context = repo.context();
    let catalog = catalog(&context);
    let before = status(&repo.root);

    let first = capture_product_package(&context, &catalog).expect("first package");
    let second = capture_product_package(&context, &catalog).expect("second package");
    assert_eq!(first.snapshot().archive(), second.snapshot().archive());
    assert_eq!(first.snapshot().inventory(), second.snapshot().inventory());
    assert_eq!(first.source_inventory(), second.source_inventory());
    assert_eq!(first.snapshot().entries().len(), 17);
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Skill)
            .count(),
        8
    );
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Agent)
            .count(),
        8
    );
    verify_product_package(&first, &context, &catalog).expect("independent verify");
    assert_eq!(status(&repo.root), before);
}

#[test]
fn legacy_skill_source_is_not_active_package_membership() {
    let repo = Repo::new("supported-package-product-legacy-excluded");
    fs::create_dir_all(repo.root.join("skills/legacy/agents")).expect("legacy metadata");
    fs::write(repo.root.join("skills/legacy/SKILL.md"), "legacy\n").expect("legacy skill");
    fs::write(
        repo.root.join("skills/legacy/agents/openai.yaml"),
        "interface: {}\n",
    )
    .expect("legacy metadata");
    let context = repo.context();
    let catalog = catalog(&context);
    let package = capture_product_package(&context, &catalog).expect("package");
    assert_eq!(package.snapshot().entries().len(), 17);
    assert!(
        package
            .snapshot()
            .entries()
            .iter()
            .all(|entry| !entry.path().starts_with("skills/legacy/"))
    );
}

#[test]
fn version_drift_and_mutate_restore_fail_closed() {
    let repo = Repo::new("supported-package-product-version-drift");
    write_draft(&repo.root, "0.0.13");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error =
        capture_product_package(&context, &authority_catalog).expect_err("version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_draft(&repo.root, SUPPORTED_VERSION);
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let session = ProductionPackageSession::begin(&context, &authority_catalog).expect("session");
    let path = repo.root.join("skills/prove/SKILL.md");
    let original = fs::read(&path).expect("original skill");
    fs::write(&path, "mutated\n").expect("mutate skill");
    fs::write(&path, original).expect("restore skill");
    let error = session.finish().expect_err("mutate restore accepted");
    assert!(matches!(
        error.id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
}
