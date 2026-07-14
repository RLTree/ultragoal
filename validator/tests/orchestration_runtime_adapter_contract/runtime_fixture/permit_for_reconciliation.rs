pub fn permit_for_reconciliation(
    authority: &RootAuthority,
    action: &RootActionRequest,
    tick: u64,
    resolution: &EffectResolution,
) -> RootPermit {
    let serial = NEXT_NONCE.fetch_add(1, Ordering::Relaxed);
    let nonce = format!("runtime-reconcile-nonce-{serial:020}");
    issue_reconcile_permit_for_test(
        authority,
        RootReconcilePermitIssuance {
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick: tick.saturating_sub(1),
            expires_tick: tick + 10,
            nonce: nonce.as_bytes(),
            target: action.target.clone(),
            resolution,
        },
    )
    .unwrap()
}

pub fn not_applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('e'),
        outcome: EffectOutcome::NotApplied,
    }
}

pub fn applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('f'),
        outcome: EffectOutcome::Applied {
            receipt: EffectReceipt {
                operation_id: "operation-001".to_owned(),
                effect: effect(),
                receipt_digest: digest('d'),
            },
        },
    }
}

pub fn state_request(head: JournalHead, tick: u64) -> OrchestrationStateRequest {
    OrchestrationStateRequest {
        expected_head: head,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
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
        touched_semantics: vec!["orchestration::runtime-adapter".to_owned()],
        generated_outputs: vec![],
        fixtures: vec![],
        effects: vec![],
        requirements: vec!["REQ-ORCH-005".to_owned()],
        dependency_nodes: vec![],
        changes: vec![BTreeMap::from([("path".to_owned(), json!(relative))])],
        commands_and_tests: vec![BTreeMap::from([
            (
                "command".to_owned(),
                json!("cargo nextest run --test orchestration_runtime_adapter_contract"),
            ),
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
        limitations: vec![],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}

pub fn recursive_fingerprint(root: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, current: &Path, rows: &mut Vec<(String, String)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
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

pub fn interrupted_root(label: &str) -> (TestRoot, JournalHead, String) {
    let (root, mut engine) = durable_engine(label, false);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"));
    let result = worker_result(&artifacts);
    let lease = lease(40);
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
    (root, head, commitment)
}

pub fn ambiguous_effect(label: &str) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, true);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let request = EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('6'),
    };
    assert_eq!(
        engine.apply_effect(3, request).unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    let head = engine.journal_head().unwrap().clone();
    (root, head)
}
