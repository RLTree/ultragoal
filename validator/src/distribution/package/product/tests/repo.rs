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
            "repository/packages/harness-ultragoal",
        )
        .expect("confined package tree")
    }

    fn journey(&self, artifact: &ProductionPackageArtifact) -> JourneyBinding {
        let host = HostCapabilityDeclaration::isolated(
            &self.root,
            &self.root,
            "macos-repository-output-v1",
            None,
        )
        .expect("host identity");
        JourneyBinding::new(
            artifact.snapshot().identity().clone(),
            &host,
            "local-harness-plugins",
        )
        .expect("journey binding")
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
        fs::create_dir_all(root.join("runtime")).expect("runtime directory");
        fs::write(root.join("schemas/catalog.json"), "{}\n").expect("schema catalog");
        write_supported_manifest(&root, SUPPORTED_VERSION);
        write_runtime_probe(&root);
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
        fs::write(root.join(".git/info/exclude"), "target/\n").expect("git output exclusion");
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

    fn workspace_context(&self) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_repository_root(&self.root)
                .expect_worktree_root(&self.root)
                .with_root_workspace_grant(&self.root),
        )
        .expect("workspace-write live context")
    }
}

fn write_runtime_probe(root: &Path) {
    let probe = root.join("runtime/runtime-probe-bin");
    fs::write(
        &probe,
        include_bytes!("../../../../../../runtime/runtime-probe-bin"),
    )
    .expect("runtime probe");
    #[cfg(unix)]
    fs::set_permissions(&probe, fs::Permissions::from_mode(0o755)).expect("runtime probe mode");
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
