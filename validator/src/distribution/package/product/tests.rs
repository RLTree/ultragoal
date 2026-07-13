use super::*;
use crate::context::{BuildRequest, LiveContext};
use crate::distribution::package::{ExpectedTree, MaterializeEffects, TreeObject, tree_sha256};
use crate::distribution::{ConfinedRoot, ScopedTree};
use crate::inventory::{AuthorityCatalog, GeneratedSurfaceIndex};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier, Mutex};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
#[cfg(unix)]
use std::os::unix::net::UnixListener;

struct Repo {
    root: PathBuf,
}

struct OutputRoot {
    root: PathBuf,
}

impl OutputRoot {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = PathBuf::from("/tmp").join(format!(
            "hul-distribution-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("confined output root");
        Self { root }
    }

    fn tree(&self) -> ScopedTree {
        ScopedTree::new(
            ConfinedRoot::open(&self.root).expect("confined output authority"),
            "candidate/package",
        )
        .expect("confined package tree")
    }
}

impl Drop for OutputRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Repo {
    fn new(label: &str) -> Self {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
        fs::create_dir_all(root.join(".codex-plugin")).expect("plugin manifest directory");
        fs::create_dir_all(root.join("schemas")).expect("schema directory");
        fs::write(root.join("schemas/catalog.json"), "{}\n").expect("schema catalog");
        write_supported_manifest(&root, SUPPORTED_VERSION);
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

fn write_supported_manifest(root: &Path, version: &str) {
    fs::write(
        root.join(SUPPORTED_MANIFEST_PATH),
        serde_json::to_vec(&json!({
            "name": PLUGIN_ID,
            "version": version,
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
    AuthorityCatalog::canonical_for_test(
        context.context_id().to_owned(),
        "test-contract".to_owned(),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("canonical test catalog")
}

#[derive(Default)]
struct MemoryOutput {
    rows: Option<Vec<TreeObject>>,
    reads: usize,
    transitions: usize,
    before_first_transition: Option<Box<dyn FnOnce()>>,
    corrupt_on_read: Option<usize>,
}

impl MaterializeEffects for MemoryOutput {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.reads += 1;
        let mut rows = self.rows.clone();
        if self.corrupt_on_read == Some(self.reads) {
            if let Some(row) = rows.as_mut().and_then(|rows| rows.first_mut()) {
                *row = TreeObject::regular(row.path().to_owned(), 0o644, b"substitute".to_vec());
            }
        }
        Ok(rows)
    }

    fn compare_exchange_tree(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        self.transitions += 1;
        if let Some(hook) = self.before_first_transition.take() {
            hook();
        }
        let current = self
            .rows
            .as_deref()
            .map(tree_sha256)
            .transpose()
            .map_err(|_| ())?;
        if current.as_deref() != expected_sha256 {
            return Ok(false);
        }
        self.rows = replacement.map(<[TreeObject]>::to_vec);
        Ok(true)
    }
}

struct SharedOutput {
    rows: Arc<Mutex<Option<Vec<TreeObject>>>>,
    first_read: Arc<Barrier>,
    reads: usize,
}

impl MaterializeEffects for SharedOutput {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        self.reads += 1;
        let observed = self.rows.lock().map_err(|_| ())?.clone();
        if self.reads == 1 {
            self.first_read.wait();
        }
        Ok(observed)
    }

    fn compare_exchange_tree(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        let mut rows = self.rows.lock().map_err(|_| ())?;
        let current = rows
            .as_deref()
            .map(tree_sha256)
            .transpose()
            .map_err(|_| ())?;
        if current.as_deref() != expected_sha256 {
            return Ok(false);
        }
        *rows = replacement.map(<[TreeObject]>::to_vec);
        Ok(true)
    }
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

fn source_tree(root: &Path) -> Vec<(String, u32, Vec<u8>)> {
    let mut rows = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry
                    .path()
                    .strip_prefix(root)
                    .ok()
                    .and_then(|path| path.components().next())
                    .is_none_or(|component| component.as_os_str() != ".git")
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
        .map(|entry| {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("source tree path")
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(path).expect("source tree metadata");
            #[cfg(unix)]
            let mode = metadata.permissions().mode();
            #[cfg(not(unix))]
            let mode = 0;
            let bytes = if metadata.is_file() {
                fs::read(path).expect("source tree file")
            } else if metadata.file_type().is_symlink() {
                fs::read_link(path)
                    .expect("source tree symlink")
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else {
                Vec::new()
            };
            (relative, mode, bytes)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn independent_product_captures_are_byte_identical_and_reverified() {
    let repo = Repo::new("supported-package-product-identical");
    let context = repo.context();
    let catalog = catalog(&context);
    let before = status(&repo.root);
    let before_tree = source_tree(&repo.root);

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
    for role in [
        PackageRole::Documentation,
        PackageRole::Executable,
        PackageRole::Data,
    ] {
        assert_eq!(
            first
                .snapshot()
                .entries()
                .iter()
                .filter(|entry| entry.role() == role)
                .count(),
            0,
            "noncanonical package role became active: {role:?}"
        );
    }
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
    assert_eq!(source_tree(&repo.root), before_tree);
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
fn undeclared_secret_local_and_benign_skill_members_fail_closed() {
    for (label, relative, bytes) in [
        (
            "secret",
            "skills/prove/.env",
            b"TOKEN=secret-package-canary\n".as_slice(),
        ),
        (
            "local",
            "skills/prove/.DS_Store",
            b"local-only\n".as_slice(),
        ),
        (
            "benign",
            "skills/prove/notes.txt",
            b"undeclared\n".as_slice(),
        ),
    ] {
        let repo = Repo::new(&format!("supported-package-product-unknown-{label}"));
        fs::write(repo.root.join(relative), bytes).expect("undeclared member");
        let context = repo.context();
        let error = capture_product_package(&context, &catalog(&context))
            .expect_err("undeclared package member accepted");
        assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);
        let diagnostic = error.to_string();
        assert!(!diagnostic.contains(relative));
        assert!(!diagnostic.contains("secret-package-canary"));
    }
}

#[test]
fn missing_or_unsafe_canonical_agent_member_fails_closed() {
    let missing = Repo::new("supported-package-product-missing-agent");
    fs::remove_file(missing.root.join("skills/prove/agents/openai.yaml"))
        .expect("remove canonical agent");
    let context = missing.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("missing canonical agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let linked = Repo::new("supported-package-product-linked-agent");
    fs::hard_link(
        linked.root.join("skills/prove/agents/openai.yaml"),
        linked.root.join("skills/prove/agents/duplicate.yaml"),
    )
    .expect("hard-linked agent");
    let context = linked.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("hard-linked canonical agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );
}

#[test]
fn manifest_absence_version_drift_and_mutate_restore_fail_closed() {
    let repo = Repo::new("supported-package-product-version-drift");
    write_draft(&repo.root, "0.0.13");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error =
        capture_product_package(&context, &authority_catalog).expect_err("version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_draft(&repo.root, SUPPORTED_VERSION);
    write_supported_manifest(&repo.root, "0.0.13");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("supported manifest version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_supported_manifest(&repo.root, SUPPORTED_VERSION);
    fs::remove_file(repo.root.join(SUPPORTED_MANIFEST_PATH)).expect("remove supported manifest");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("missing supported manifest accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);

    write_supported_manifest(&repo.root, SUPPORTED_VERSION);
    fs::remove_file(repo.root.join("plugin-manifest-draft.json")).expect("remove draft manifest");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("missing draft manifest accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);

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

#[test]
fn publication_is_current_bound_and_reconciles_one_complete_pair() {
    let repo = Repo::new("supported-package-product-publication");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let before = status(&repo.root);
    let before_tree = source_tree(&repo.root);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let output_root = OutputRoot::new("supported-package-product-publication");
    let mut output = output_root.tree();

    let transaction = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect("current-bound publication");
    let rows = output
        .inspect(2, 65 * 1024 * 1024)
        .expect("inspect package output")
        .expect("published artifact pair");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        tree_sha256(&rows).unwrap(),
        transaction.output_tree_sha256()
    );
    assert!(rows.iter().any(|row| row.path().ends_with(".hugpkg")));
    assert!(
        rows.iter()
            .any(|row| row.path().ends_with(".inventory.json"))
    );
    verify_product_package(&artifact, &context, &authority_catalog).expect("post-publish verify");
    assert_eq!(status(&repo.root), before);
    assert_eq!(source_tree(&repo.root), before_tree);
}

#[test]
fn stale_candidate_or_forged_catalog_is_rejected_before_output_effects() {
    let repo = Repo::new("supported-package-product-stale-publication");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    fs::write(repo.root.join("skills/prove/SKILL.md"), "changed\n").expect("candidate drift");
    let changed_context = repo.context();
    let changed_catalog = catalog(&changed_context);
    let mut output = MemoryOutput::default();
    let error = artifact
        .publish(
            &changed_context,
            &changed_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("stale artifact published");
    assert_eq!(error.id(), ProductionPackageErrorId::CatalogMismatch);
    assert_eq!((output.reads, output.transitions), (0, 0));

    let current_artifact =
        capture_product_package(&changed_context, &changed_catalog).expect("current package");
    let forged_catalog = AuthorityCatalog::new(
        format!("sha256:{}", "f".repeat(64)),
        changed_context.context_id().to_owned(),
        "test-contract".to_owned(),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        GeneratedSurfaceIndex::new(Vec::new()),
    );
    let error = current_artifact
        .publish(
            &changed_context,
            &forged_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("forged catalog published");
    assert_eq!(error.id(), ProductionPackageErrorId::CatalogMismatch);
    assert_eq!((output.reads, output.transitions), (0, 0));
}

#[test]
fn source_mutate_restore_during_publication_rolls_back_output() {
    let repo = Repo::new("supported-package-product-publication-mutate-restore");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let path = repo.root.join("skills/prove/SKILL.md");
    let original = fs::read(&path).expect("original source");
    let hook_path = path.clone();
    let hook_original = original.clone();
    let mut output = MemoryOutput {
        before_first_transition: Some(Box::new(move || {
            fs::write(&hook_path, b"mutated during output\n").expect("mutate source");
            fs::write(&hook_path, hook_original).expect("restore source");
        })),
        ..MemoryOutput::default()
    };

    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("mutate-restore crossed publication");
    assert!(matches!(
        error.id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
    assert!(
        output.rows.is_none(),
        "failed publication was not rolled back"
    );
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn two_concurrent_publishers_yield_one_exact_winner() {
    let repo = Repo::new("supported-package-product-writer-race");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let rows = Arc::new(Mutex::new(None));
    let first_read = Arc::new(Barrier::new(2));

    let handles = (0..2)
        .map(|_| {
            let context = context.clone();
            let authority_catalog = authority_catalog.clone();
            let artifact = artifact.clone();
            let rows = Arc::clone(&rows);
            let first_read = Arc::clone(&first_read);
            std::thread::spawn(move || {
                let mut output = SharedOutput {
                    rows,
                    first_read,
                    reads: 0,
                };
                artifact.publish(
                    &context,
                    &authority_catalog,
                    &ExpectedTree::Absent,
                    &mut output,
                )
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("publisher thread"))
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        1
    );
    assert_eq!(rows.lock().unwrap().as_ref().unwrap().len(), 2);
}

#[test]
fn partial_pair_and_failed_output_reconciliation_never_publish() {
    let repo = Repo::new("supported-package-product-output-false-pass");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let partial = vec![TreeObject::regular(
        format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.hugpkg"),
        0o644,
        artifact.snapshot().archive().to_vec(),
    )];
    let partial_sha256 = tree_sha256(&partial).expect("partial digest");
    let mut partial_output = MemoryOutput {
        rows: Some(partial.clone()),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::ExactDigest(partial_sha256),
            &mut partial_output,
        )
        .expect_err("partial pair was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(partial_output.rows, Some(partial));
    assert_eq!(partial_output.transitions, 0);

    let inventory_only = vec![TreeObject::regular(
        format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.inventory.json"),
        0o644,
        artifact.snapshot().inventory().to_vec(),
    )];
    let inventory_only_sha256 = tree_sha256(&inventory_only).expect("inventory-only digest");
    let mut inventory_only_output = MemoryOutput {
        rows: Some(inventory_only.clone()),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::ExactDigest(inventory_only_sha256),
            &mut inventory_only_output,
        )
        .expect_err("inventory-only output was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(inventory_only_output.rows, Some(inventory_only));
    assert_eq!(inventory_only_output.transitions, 0);

    let mut corrupt_output = MemoryOutput {
        corrupt_on_read: Some(2),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::Absent,
            &mut corrupt_output,
        )
        .expect_err("corrupt post-write pair was accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert!(corrupt_output.rows.is_none());
}

#[test]
fn same_version_different_bytes_cannot_replace_published_pair() {
    let repo = Repo::new("supported-package-product-same-version-reuse");
    let first_context = repo.context();
    let first_catalog = catalog(&first_context);
    let first = capture_product_package(&first_context, &first_catalog).expect("first package");
    let output_root = OutputRoot::new("supported-package-product-same-version");
    let mut output = output_root.tree();
    let transaction = first
        .publish(
            &first_context,
            &first_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect("first publish");
    let original_rows = output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();

    fs::write(repo.root.join("skills/prove/SKILL.md"), "different bytes\n")
        .expect("mutate product source");
    let second_context = repo.context();
    let second_catalog = catalog(&second_context);
    let second = capture_product_package(&second_context, &second_catalog).expect("second package");
    assert_ne!(first.snapshot().archive(), second.snapshot().archive());
    let error = second
        .publish(
            &second_context,
            &second_catalog,
            &ExpectedTree::ExactDigest(transaction.output_tree_sha256().to_owned()),
            &mut output,
        )
        .expect_err("same-version different bytes replaced output");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(
        output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap(),
        original_rows
    );
}

#[test]
fn every_artifact_binding_dimension_is_checked_before_output_effects() {
    let repo = Repo::new("supported-package-product-binding-substitution");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");

    let mut substitutions = Vec::new();
    let mut candidate = artifact.clone();
    candidate.candidate_id = format!("sha256:{}", "1".repeat(64));
    substitutions.push(candidate);
    let mut artifact_catalog = artifact.clone();
    artifact_catalog.catalog_id = format!("sha256:{}", "2".repeat(64));
    substitutions.push(artifact_catalog);
    let mut source_inventory = artifact.clone();
    source_inventory.source_inventory.push(b' ');
    substitutions.push(source_inventory);
    let mut source_snapshot = artifact.clone();
    source_snapshot.source_snapshot_id = format!("sha256:{}", "3".repeat(64));
    substitutions.push(source_snapshot);
    let mut plan_context = artifact.clone();
    plan_context.plan.context_id = format!("sha256:{}", "4".repeat(64));
    substitutions.push(plan_context);
    let mut plan_version = artifact.clone();
    plan_version.plan.version = "0.0.13".to_string();
    substitutions.push(plan_version);
    let mut plan_catalog = artifact.clone();
    plan_catalog.plan.catalog_id = format!("sha256:{}", "5".repeat(64));
    substitutions.push(plan_catalog);
    let mut plan_inventory = artifact.clone();
    plan_inventory.plan.accepted_inventory_sha256 = format!("sha256:{}", "6".repeat(64));
    substitutions.push(plan_inventory);
    let mut plan_tree = artifact.clone();
    plan_tree.plan.source_tree_sha256 = format!("sha256:{}", "7".repeat(64));
    substitutions.push(plan_tree);
    let mut plan_mode = artifact.clone();
    plan_mode.plan.entries[0].mode = 0o755;
    substitutions.push(plan_mode);
    let mut plan_row = artifact.clone();
    plan_row.plan.entries[0].bytes.push(b' ');
    substitutions.push(plan_row);
    let mut binding = artifact.clone();
    binding.binding = binding
        .binding
        .substituted("inventory_sha256", &format!("sha256:{}", "8".repeat(64)));
    substitutions.push(binding);

    for altered in substitutions {
        let mut output = MemoryOutput::default();
        assert!(
            altered
                .publish(
                    &context,
                    &authority_catalog,
                    &ExpectedTree::Absent,
                    &mut output,
                )
                .is_err()
        );
        assert_eq!((output.reads, output.transitions), (0, 0));
    }
}

#[test]
fn case_collision_symlink_special_file_and_unsafe_mode_fail_closed() {
    let case_collision = Repo::new("supported-package-product-case-collision");
    let alias_root = case_collision.root.join("skills/PROVE");
    match fs::create_dir(&alias_root) {
        Ok(()) => {
            fs::write(alias_root.join("SKILL.md"), "case alias\n").expect("case alias file");
            let context = case_collision.context();
            assert_eq!(
                capture_product_package(&context, &catalog(&context))
                    .expect_err("case collision accepted")
                    .id(),
                ProductionPackageErrorId::SourceUnavailable
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // The host filesystem cannot represent a distinct ASCII case-colliding entry.
            // The synthetic capture control covers the production classifier on this host.
        }
        Err(error) => panic!("case alias directory: {error}"),
    }

    let linked = Repo::new("supported-package-product-symlink");
    let agent = linked.root.join("skills/prove/agents/openai.yaml");
    fs::remove_file(&agent).expect("remove canonical agent");
    symlink(linked.root.join("skills/prove/SKILL.md"), &agent).expect("symlink agent");
    let context = linked.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("symlink accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let special = Repo::new("supported-package-product-special");
    let agent = special.root.join("skills/prove/agents/openai.yaml");
    fs::remove_file(&agent).expect("remove canonical agent");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let socket_path =
        PathBuf::from("/tmp").join(format!("hul-pkg-{}-{nonce}.sock", std::process::id()));
    let _listener = UnixListener::bind(&socket_path).expect("short special socket");
    fs::rename(&socket_path, &agent).expect("move special socket into canonical member");
    let context = special.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("special file accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let executable = Repo::new("supported-package-product-unsafe-mode");
    let skill = executable.root.join("skills/prove/SKILL.md");
    let mut permissions = fs::metadata(&skill).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&skill, permissions).expect("set executable mode");
    let context = executable.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("unsafe mode accepted")
            .id(),
        ProductionPackageErrorId::MembershipMismatch
    );
}

#[test]
fn duplicate_json_path_escape_and_oversized_source_fail_closed() {
    let duplicate = Repo::new("supported-package-product-duplicate-json");
    fs::write(
        duplicate.root.join(SUPPORTED_MANIFEST_PATH),
        br#"{"name":"harness-ultragoal","name":"harness-ultragoal"}"#,
    )
    .expect("duplicate-key manifest");
    let context = duplicate.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("duplicate JSON accepted")
            .id(),
        ProductionPackageErrorId::ManifestMismatch
    );

    let escaped = Repo::new("supported-package-product-path-escape");
    let draft_path = escaped.root.join("plugin-manifest-draft.json");
    let mut draft: serde_json::Value =
        serde_json::from_slice(&fs::read(&draft_path).unwrap()).unwrap();
    draft["skills"][0]["path"] = json!("../canary/SKILL.md");
    fs::write(&draft_path, serde_json::to_vec(&draft).unwrap()).expect("escaped draft");
    let context = escaped.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("path escape accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let oversized = Repo::new("supported-package-product-oversized-entry");
    fs::OpenOptions::new()
        .write(true)
        .open(oversized.root.join("skills/prove/SKILL.md"))
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    let context = oversized.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("oversized package entry accepted")
            .id(),
        ProductionPackageErrorId::ArchiveMismatch
    );
}
