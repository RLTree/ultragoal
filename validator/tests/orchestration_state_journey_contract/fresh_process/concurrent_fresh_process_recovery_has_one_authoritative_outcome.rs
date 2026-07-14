#[test]
fn concurrent_fresh_process_recovery_has_one_authoritative_outcome() {
    let (journal, mut engine) = durable_engine("fresh-recover", false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let event = engine
        .event_log()
        .next(
            &binding(),
            worker_actor(),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    drop(engine);
    let mut bytes = fs::read(journal.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(journal.path().join("events.jsonl"), bytes).unwrap();
    let workspace_identity = ProductWorkspace::open(journal.path())
        .unwrap()
        .identity()
        .to_owned();
    let control = TestRoot::new("fresh-recover-control");

    let mut inputs = Vec::new();
    for index in 0..2 {
        let output = control.path().join(format!("recover-{index}-output.json"));
        let mut request = input("recover", &journal, output.clone(), prior.clone(), 3);
        request.expected_event_id = Some(event.event_id.clone());
        request.recovered_binding = Some(binding());
        let path = write_input(&control, &format!("recover-{index}"), &request);
        inputs.push((path, output));
    }
    let mut children = inputs
        .iter()
        .map(|(path, _)| spawn_child(path))
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let outputs = inputs
        .iter()
        .map(|(_, output)| {
            serde_json::from_slice::<ChildOutput>(&fs::read(output).unwrap()).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status == "recovered")
            .count(),
        1
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.error_code.as_deref() == Some("HUL-ORCH-PROD-005"))
            .count(),
        1
    );
    let recovered = outputs
        .iter()
        .find(|output| output.status == "recovered")
        .unwrap();
    assert_eq!(
        recovered.workspace_identity.as_deref(),
        Some(workspace_identity.as_str())
    );
    assert_eq!(recovered.snapshot.as_ref().unwrap().binding, binding());
    let committed = FileJournal::open(journal.path())
        .unwrap()
        .inspect()
        .unwrap();
    assert_eq!(committed.head, *recovered.current_head.as_ref().unwrap());
    assert_eq!(committed.head.event_count, prior.event_count + 1);
}

fn journal_frame_bytes(event: &OrchestrationEvent) -> Vec<u8> {
    #[derive(Serialize)]
    struct Frame<'a> {
        schema_version: &'static str,
        event: &'a OrchestrationEvent,
    }
    serde_json::to_vec(&Frame {
        schema_version: "OrchestrationJournalFrame-v1",
        event,
    })
    .unwrap()
}
