#[test]
fn failures_never_echo_attacker_controlled_input() {
    let canary = "SECRET_PATH_CANARY";
    let error = CanonicalPath::parse(&format!("../{canary}\n")).unwrap_err();
    let rendered = error.to_string();
    assert!(!rendered.contains(canary));
    assert_eq!(
        rendered,
        "HUL-ORCH-003: path is not a confined relative path"
    );
}

#[test]
fn path_parser_rejects_escape_and_nonportable_forms() {
    for candidate in [
        "../escape",
        "/absolute",
        "a/./b",
        "a//b",
        "C:\\windows",
        "a:b",
        "line\nbreak",
        ".",
        "~",
    ] {
        assert_eq!(
            CanonicalPath::parse(candidate).unwrap_err(),
            OrchestrationError::InvalidPath
        );
    }
}

#[test]
fn canonical_path_deserialization_rejects_every_noncanonical_form_without_echo() {
    let oversized = "a".repeat(513);
    let candidates = [
        "validator/src/orchestration/../../Cargo.lock",
        "/absolute",
        "C:\\windows",
        "a//b",
        "a/./b",
        "line\nbreak",
        "nul\0byte",
        "caf\u{e9}",
        oversized.as_str(),
        "../SECRET_DESERIALIZE_CANARY",
    ];

    for candidate in candidates {
        let error = serde_json::from_value::<CanonicalPath>(serde_json::json!(candidate))
            .expect_err("noncanonical path must fail during deserialization");
        assert!(!error.to_string().contains(candidate));
        assert!(!error.to_string().contains("SECRET_DESERIALIZE_CANARY"));
    }
}

#[test]
fn lease_and_event_json_substitution_cannot_bypass_path_validation() {
    let oversized = "a".repeat(513);
    let candidates = [
        "validator/src/orchestration/../../Cargo.lock",
        "/absolute",
        "C:\\windows",
        "a//b",
        "a/./b",
        "line\nbreak",
        "nul\0byte",
        "caf\u{e9}",
        oversized.as_str(),
    ];
    let locations = [
        "/read_paths",
        "/owned_scope/paths",
        "/owned_scope/generated_outputs",
        "/owned_scope/fixtures",
    ];

    for candidate in candidates {
        for location in locations {
            let mut encoded = serde_json::to_value(lease()).unwrap();
            *encoded.pointer_mut(location).unwrap() = serde_json::json!([candidate]);
            assert!(serde_json::from_value::<LeaseSpec>(encoded).is_err());
        }

        let event = OrchestrationEvent::create(
            0,
            None,
            binding(),
            root(),
            1,
            EventKind::LeaseGranted { lease: lease() },
        )
        .unwrap();
        let mut encoded = serde_json::to_value(event).unwrap();
        encoded["event"]["lease"]["owned_scope"]["paths"] = serde_json::json!([candidate]);
        assert!(serde_json::from_value::<OrchestrationEvent>(encoded).is_err());
    }
}

#[test]
fn canonical_path_lease_and_event_roundtrip_remain_lossless() {
    let original = lease();
    let encoded = serde_json::to_vec(&original).unwrap();
    assert_eq!(
        serde_json::from_slice::<LeaseSpec>(&encoded).unwrap(),
        original
    );

    let event = OrchestrationEvent::create(
        0,
        None,
        binding(),
        root(),
        1,
        EventKind::LeaseGranted { lease: lease() },
    )
    .unwrap();
    let encoded = serde_json::to_vec(&event).unwrap();
    assert_eq!(
        serde_json::from_slice::<OrchestrationEvent>(&encoded).unwrap(),
        event
    );
}

#[test]
fn digest_and_actor_parsers_reject_forged_identifiers() {
    assert_eq!(
        Binding::new("sha256:short", &digest('a')).unwrap_err(),
        OrchestrationError::InvalidDigest
    );
    assert_eq!(
        Actor::parse("worker secret\ncanary").unwrap_err(),
        OrchestrationError::InvalidIdentifier
    );
    assert_eq!(
        Actor::parse("/root/n10_orchestration").unwrap().as_str(),
        "/root/n10_orchestration"
    );
    assert_eq!(
        Actor::parse("/root//forged").unwrap_err(),
        OrchestrationError::InvalidIdentifier
    );
}

#[test]
fn root_authority_effect_is_never_worker_leaseable() {
    let mut owned = scope("node_a");
    owned.effects = [effect(EffectClass::RootAuthority, "claim-decision")].into();
    let mut expanded = policy();
    expanded
        .allowed_effects
        .insert(effect(EffectClass::RootAuthority, "claim-decision"));
    assert_eq!(
        lease_with_scope("lease-001", "node-a", "worker-a", owned)
            .validate(&expanded)
            .unwrap_err(),
        OrchestrationError::RootOnlyScope
    );
}

#[test]
fn oversized_worker_result_is_rejected_before_json_parse() {
    let bytes = vec![b' '; 4 * 1024 * 1024 + 1];
    assert_eq!(
        WorkerResultV1::parse_json(&bytes).unwrap_err(),
        OrchestrationError::ResourceLimit
    );
}

#[test]
fn event_substitution_is_detected_even_if_shape_is_valid() {
    let event =
        OrchestrationEvent::create(0, None, binding(), root(), 1, EventKind::RootInterrupted)
            .unwrap();
    let mut encoded = serde_json::to_value(event).unwrap();
    encoded["logical_tick"] = serde_json::json!(2);
    let changed: OrchestrationEvent = serde_json::from_value(encoded).unwrap();
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::restart(
            graph_one(),
            policy(),
            binding(),
            root(),
            EventLog(vec![changed]),
            sink,
        )
        .err()
        .unwrap(),
        OrchestrationError::InvalidEvent
    );
}
