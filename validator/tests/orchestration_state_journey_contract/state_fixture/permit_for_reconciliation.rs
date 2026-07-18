pub fn state_request(head: JournalHead, tick: u64) -> command::OrchestrationStateRequest {
    command::OrchestrationStateRequest {
        expected_head: head,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
}

pub fn recursive_fingerprint(root: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, current: &Path, rows: &mut Vec<(String, String)>) {
        let mut paths = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                rows.push((relative, "directory".to_owned()));
                walk(base, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((relative, "symlink".to_owned()));
            } else if metadata.is_file() {
                rows.push((relative, content_digest(&fs::read(&path).unwrap())));
            } else {
                rows.push((relative, "special".to_owned()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

pub fn worker_result(artifacts: &TestRoot) -> WorkerResultV1 {
    let relative = "work/node-a/output.json";
    let bytes = b"{\"result\":\"bounded\"}\n";
    let target = artifacts.path().join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, bytes).unwrap();
    WorkerResultV1 {
        worker: "worker-a".to_owned(),
        lease_id: "lease-001".to_owned(),
        context_id: binding().context_id.clone(),
        candidate_identity: BTreeMap::from([
            (
                "candidate_id".to_owned(),
                Value::String(binding().candidate_id),
            ),
            ("context_id".to_owned(), Value::String(binding().context_id)),
        ]),
        base_state: BTreeMap::from([("status".to_owned(), json!("leased"))]),
        final_state: BTreeMap::from([(
            "status".to_owned(),
            json!("candidate_for_root_acceptance"),
        )]),
        touched_paths: vec![relative.to_owned()],
        touched_semantics: vec!["orchestration::journey::node-a".to_owned()],
        generated_outputs: vec![],
        fixtures: vec![],
        effects: vec![],
        requirements: vec!["REQ-ORCH-005".to_owned()],
        dependency_nodes: vec![],
        changes: vec![BTreeMap::from([("path".to_owned(), json!(relative))])],
        commands_and_tests: vec![BTreeMap::from([
            ("command".to_owned(), json!("cargo nextest run focused")),
            ("status".to_owned(), json!("pass")),
        ])],
        artifacts: vec![ArtifactRecord {
            path: relative.to_owned(),
            sha256: content_digest(bytes),
            byte_length: bytes.len() as u64,
        }],
        findings: vec![],
        unresolved_dependencies: vec![],
        requested_root_changes: vec![],
        limitations: vec!["root-reconciliation-pending".to_owned()],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}
