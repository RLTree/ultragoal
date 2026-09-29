pub fn submitted_interrupted(label: &str) -> (TestRoot, TestRoot, JournalHead, String) {
    let journal = TestRoot::new(label, 0o700);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"), 0o700);
    let mut engine = engine(&journal);
    let lease = lease(40);
    let result = submitted_result(&artifacts);
    engine.grant_lease(1, lease.clone()).unwrap();
    engine.start(2, &lease.lease_id).unwrap();
    engine
        .submit(
            3,
            &lease.lease_id,
            &result,
            &ArtifactWorkspace::new(artifacts.path()).unwrap(),
        )
        .unwrap();
    let commitment = engine
        .events()
        .iter()
        .find_map(|event| match &event.event {
            EventKind::WorkerSubmitted {
                result_commitment_id,
                ..
            } => Some(result_commitment_id.clone()),
            _ => None,
        })
        .unwrap();
    engine.interrupt_root(4).unwrap();
    let head = engine.journal_head().unwrap().clone();
    (journal, artifacts, head, commitment)
}

fn submitted_result(artifacts: &TestRoot) -> WorkerResultV1 {
    let relative = "work/node-a/output.json";
    let bytes = b"{\"result\":\"bounded\"}\n";
    let output = artifacts.path().join(relative);
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, bytes).unwrap();
    WorkerResultV1 {
        worker: "worker-a".to_owned(),
        lease_id: "lease-001".to_owned(),
        context_id: binding().context_id.clone(),
        candidate_identity: BTreeMap::from([
            (
                "candidate_id".to_owned(),
                serde_json::Value::String(binding().candidate_id),
            ),
            (
                "context_id".to_owned(),
                serde_json::Value::String(binding().context_id),
            ),
        ]),
        base_state: BTreeMap::from([("status".to_owned(), serde_json::json!("leased"))]),
        final_state: BTreeMap::from([(
            "status".to_owned(),
            serde_json::json!("candidate_for_root_acceptance"),
        )]),
        touched_paths: vec![relative.to_owned()],
        touched_semantics: vec!["orchestration::product::node-a".to_owned()],
        generated_outputs: vec![],
        fixtures: vec![],
        effects: vec![EffectUse {
            class: EffectClass::Process,
            target: "worker-effect".to_owned(),
            performed: true,
        }],
        requirements: vec!["REQ-ORCH-005".to_owned()],
        dependency_nodes: vec![],
        changes: vec![BTreeMap::from([(
            "path".to_owned(),
            serde_json::json!(relative),
        )])],
        commands_and_tests: vec![BTreeMap::from([
            ("command".to_owned(), serde_json::json!("focused-control")),
            ("status".to_owned(), serde_json::json!("pass")),
        ])],
        artifacts: vec![ArtifactRecord {
            path: relative.to_owned(),
            sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
            byte_length: bytes.len() as u64,
        }],
        findings: vec![],
        unresolved_dependencies: vec![],
        requested_root_changes: vec![],
        limitations: vec![],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}
