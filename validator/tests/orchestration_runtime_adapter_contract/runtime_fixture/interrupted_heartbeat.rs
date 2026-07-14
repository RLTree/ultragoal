pub fn interrupted_heartbeat(
    label: &str,
) -> (
    TestRoot,
    JournalHead,
    OrchestrationEvent,
    InterruptedRecoveryRequest,
) {
    let (root, mut engine) = durable_engine(label, false);
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
    let mut bytes = fs::read(root.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();
    let request = InterruptedRecoveryRequest {
        expected_prior_head: prior.clone(),
        expected_event_id: event.event_id.clone(),
        recovered_binding: binding(),
        tick: 3,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    (root, prior, event, request)
}

pub fn journal_frame_bytes(event: &OrchestrationEvent) -> Vec<u8> {
    #[derive(serde::Serialize)]
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
