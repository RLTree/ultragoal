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
