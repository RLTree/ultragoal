const CATALOG: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

#[derive(Serialize)]
struct Inventory<'a> {
    schema: &'static str,
    context_id: &'static str,
    candidate_id: &'static str,
    catalog_id: &'static str,
    plugin_id: &'static str,
    version: &'static str,
    source_date_epoch: u64,
    entries: Vec<Entry<'a>>,
}

#[derive(Serialize)]
struct Entry<'a> {
    path: &'a str,
    object_type: &'static str,
    mode: u32,
    sha256: String,
    byte_length: u64,
    role: &'static str,
}

fn source_fixture(label: &str) -> (Fixture, Vec<u8>) {
    let fixture = Fixture::complete(label);
    let files: [(&str, Vec<u8>, &str); 3] = [
        (
            ".codex-plugin/plugin.json",
            serde_json::to_vec(&manifest()).unwrap(),
            "manifest",
        ),
        (
            "skills/harness-ultragoal/SKILL.md",
            b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n".to_vec(),
            "skill",
        ),
        (
            "skills/prove/SKILL.md",
            b"---\nname: prove\ndescription: Proof workflow\n---\n".to_vec(),
            "skill",
        ),
    ];
    for (path, bytes, _) in &files {
        fs::create_dir_all(fixture.root.join(path).parent().unwrap()).unwrap();
        fs::write(fixture.root.join(path), bytes).unwrap();
    }
    let entries = files
        .iter()
        .map(|(path, bytes, role)| Entry {
            path,
            object_type: "regular-file",
            mode: 0o644,
            sha256: digest(bytes),
            byte_length: bytes.len() as u64,
            role,
        })
        .collect();
    let inventory = serde_json::to_vec(&Inventory {
        schema: "harness-ultragoal.accepted-package-source-set.v1",
        context_id: CONTEXT_ID,
        candidate_id: CANDIDATE_ID,
        catalog_id: CATALOG,
        plugin_id: "harness-ultragoal",
        version: "0.0.11",
        source_date_epoch: 1_700_000_000,
        entries,
    })
    .unwrap();
    (fixture, inventory)
}

#[derive(Default)]
struct ArchiveSink(Option<Vec<u8>>);

impl PackageEffects for ArchiveSink {
    fn read_package(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }
    fn compare_exchange_package(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        if self.0.as_deref().map(digest).as_deref() != expected {
            return Ok(false);
        }
        self.0 = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}

#[derive(Default)]
struct TreeSink {
    objects: Option<Vec<TreeObject>>,
    race: Option<Vec<TreeObject>>,
    corrupt_after_write: bool,
    transitions: usize,
}

impl MaterializeEffects for TreeSink {
    fn read_tree(&mut self, _: usize, _: usize) -> Result<Option<Vec<TreeObject>>, ()> {
        let mut rows = self.objects.clone();
        if self.corrupt_after_write && self.transitions > 0 {
            if let Some(first) = rows.as_mut().and_then(|value| value.first_mut()) {
                *first = TreeObject::regular(first.path().into(), 0o644, b"substitute".to_vec());
            }
        }
        Ok(rows)
    }
    fn compare_exchange_tree(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[TreeObject]>,
    ) -> Result<bool, ()> {
        self.transitions += 1;
        if let Some(race) = self.race.take() {
            self.objects = Some(race);
        }
        let current = self
            .objects
            .as_deref()
            .map(tree_sha256)
            .transpose()
            .map_err(|_| ())?;
        if current.as_deref() != expected {
            return Ok(false);
        }
        self.objects = replacement.map(<[TreeObject]>::to_vec);
        Ok(true)
    }
}

#[test]
fn accepted_inventory_double_build_and_materialized_tree_are_deterministic() {
    let (fixture, inventory) = source_fixture("accepted-package-positive");
    let before = tree(&fixture.root);
    let one = plan_package_from_inventory(&fixture.root, &inventory).unwrap();
    let two = plan_package_from_inventory(&fixture.root, &inventory).unwrap();
    assert_eq!(tree(&fixture.root), before, "planning must be zero-write");
    assert_eq!(one.source_tree_sha256(), two.source_tree_sha256());
    assert_eq!(one.accepted_inventory_sha256(), digest(&inventory));
    let first = build_package(&one, &mut ArchiveSink::default()).unwrap();
    let second = build_package(&two, &mut ArchiveSink::default()).unwrap();
    assert_eq!(first.archive(), second.archive());
    assert_eq!(first.inventory(), second.inventory());
    assert_eq!(first.identity().source().catalog_id(), CATALOG);
    assert_eq!(
        first.identity().source().accepted_inventory_sha256(),
        digest(&inventory)
    );
    let mut destination = TreeSink::default();
    let transaction = materialize_package(&one, &ExpectedTree::Absent, &mut destination).unwrap();
    assert_eq!(transaction.tree_sha256(), one.source_tree_sha256());
    rollback_materialization(transaction, &mut destination).unwrap();
    assert!(destination.objects.is_none());
}

#[test]
fn source_mutation_unknown_rows_traversal_and_stale_digest_fail_closed() {
    let (fixture, inventory) = source_fixture("accepted-package-negative");
    let mut value: serde_json::Value = serde_json::from_slice(&inventory).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert_eq!(
        plan_package_from_inventory(&fixture.root, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::InvalidSpec
    );
    let mut value: serde_json::Value = serde_json::from_slice(&inventory).unwrap();
    value["entries"][1]["path"] = serde_json::json!("../escape");
    assert_eq!(
        plan_package_from_inventory(&fixture.root, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::InvalidPath
    );
    fs::write(
        fixture.root.join("skills/prove/SKILL.md"),
        b"mutated source",
    )
    .unwrap();
    assert_eq!(
        plan_package_from_inventory(&fixture.root, &inventory)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged
    );
}
