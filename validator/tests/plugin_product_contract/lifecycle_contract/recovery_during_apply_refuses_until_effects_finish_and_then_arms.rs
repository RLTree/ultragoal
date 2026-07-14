#[test]
fn recovery_during_apply_refuses_until_effects_finish_and_then_arms() {
    let current = authority("0.0.12", D1);
    let before = installed(current.clone());
    let update = plan(
        &before,
        &request(
            LifecycleIntent::MonotonicUpdate,
            Some(authority("0.0.13", D2)),
            auth(Some(&current)),
        ),
    )
    .unwrap();
    let token = recovery_token(&update).unwrap();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let apply_thread = {
        let before = before.clone();
        let update = update.clone();
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        std::thread::spawn(move || {
            let mut adapter = BlockingAdapter {
                effects: Vec::new(),
                entered,
                release,
            };
            let report = apply(&before, &update, &mut adapter);
            (report, adapter.effects)
        })
    };
    entered.wait();
    let mut concurrent = Adapter::default();
    assert_eq!(
        recover(&update.expected_after, &token, &mut concurrent),
        Err(LifecycleError::RecoveryUnavailable)
    );
    assert!(concurrent.restored.is_empty());
    release.wait();
    let (report, effects) = apply_thread.join().unwrap();
    let report = report.unwrap();
    assert_eq!(effects, update.effects);
    let mut recovery = Adapter::default();
    assert_eq!(
        recover(&report.state, &token, &mut recovery).unwrap(),
        before
    );
    assert_eq!(recovery.restored, vec![before]);
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleCases {
    schema_version: String,
    cases: Vec<LifecycleCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleCase {
    id: String,
    intent: LifecycleIntent,
    expected_effects: Vec<LifecycleEffect>,
    prior_authority_preserved_on_failure: bool,
}

#[test]
fn lifecycle_fixture_is_exact_and_covers_all_eight_intents() {
    let fixture: LifecycleCases =
        serde_json::from_str(&super::read("fixtures/plugin-product/lifecycle-cases.json")).unwrap();
    assert_eq!(fixture.schema_version, "HarnessPluginLifecycleCases-v1");
    assert_eq!(fixture.cases.len(), 8);
    for (case, journey) in fixture.cases.iter().zip(JOURNEYS.iter()) {
        assert_eq!(case.id, journey.id);
        assert_eq!(case.intent, journey.intent);
        assert_eq!(case.expected_effects, journey.required_effects);
        assert!(case.prior_authority_preserved_on_failure);
    }
}

#[test]
fn lifecycle_json_rejects_unknown_fields_and_invalid_versions() {
    assert!(serde_json::from_str::<LifecycleRequest>(
        r#"{"intent":"fresh_install","target":null,"prior_authority":null,"authorization":{"allow_host_write":false,"allow_downgrade":false,"expected_installed_sha256":null},"unknown":true}"#
    ).is_err());
    for value in ["1", "1.2", "1.2.3.4", "01.2.3", "a.2.3"] {
        assert_eq!(Version::parse(value), Err(LifecycleError::InvalidVersion));
    }
}
