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
    scope: String,
    public_authority: bool,
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
    assert_eq!(fixture.schema_version, "HarnessPluginLifecycleCases-v2");
    assert_eq!(fixture.scope, "disposable_non_personal_verifier_fixture");
    assert!(!fixture.public_authority);
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

#[test]
fn lifecycle_versions_accept_only_normalized_codex_cachebusters_for_stable_releases() {
    let installed_version = Version::parse("0.0.39+codex.20260820190706").unwrap();
    let target = Version::parse("0.0.41+codex.20260824093100").unwrap();
    let bare = Version::parse("0.0.39").unwrap();
    let alternate = Version::parse("0.0.39+codex.local-20260824-120000").unwrap();
    assert_ne!(installed_version, bare);
    assert_ne!(installed_version, alternate);
    assert_eq!(
        installed_version.precedence_cmp(&bare).unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        installed_version.precedence_cmp(&alternate).unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        target.precedence_cmp(&installed_version).unwrap(),
        std::cmp::Ordering::Greater
    );

    for value in [
        "0.0.39+other.20260820190706",
        "0.0.39+codex.",
        "0.0.39+codex.-token",
        "0.0.39+codex.token-",
        "0.0.39+codex.two--hyphens",
        "0.0.39+codex.UPPER",
        "0.0.39+codex.two.parts",
        "0.0.39+codex.token+extra",
    ] {
        assert_eq!(Version::parse(value), Err(LifecycleError::InvalidVersion));
    }

    let before = installed(authority("0.0.39+codex.20260820190706", D1));
    assert_eq!(
        plan(
            &before,
            &request(
                LifecycleIntent::MonotonicUpdate,
                Some(authority("0.0.39+codex.different-cachebuster", D2)),
                auth(Some(before.installed.as_ref().unwrap())),
            ),
        ),
        Err(LifecycleError::InvalidTransition)
    );
}

#[test]
fn lifecycle_version_serialization_preserves_legacy_shape_and_semver_parts() {
    let legacy = Version::parse("1.2.3").unwrap();
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::json!({"major":1,"minor":2,"patch":3})
    );
    assert_eq!(
        serde_json::from_value::<Version>(serde_json::json!({
            "major": 1,
            "minor": 2,
            "patch": 3,
        }))
        .unwrap(),
        legacy
    );

    let prerelease = Version::parse("1.2.3-beta.2+codex.local-20260824-120000").unwrap();
    let later = Version::parse("1.2.3-beta.11+codex.local-20260824-120001").unwrap();
    assert_eq!(
        prerelease.precedence_cmp(&later).unwrap(),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        prerelease.to_string(),
        "1.2.3-beta.2+codex.local-20260824-120000"
    );
    assert_eq!(
        serde_json::to_value(&prerelease).unwrap(),
        serde_json::json!({
            "major": 1,
            "minor": 2,
            "patch": 3,
            "prerelease": "beta.2",
            "build_metadata": "codex.local-20260824-120000",
        })
    );

    let forged: PackageAuthority = serde_json::from_value(serde_json::json!({
        "version": {
            "major": 1,
            "minor": 2,
            "patch": 3,
            "build_metadata": "other.substituted",
        },
        "package_sha256": D1,
        "inventory_sha256": D2,
        "candidate_id": D3,
    }))
    .unwrap();
    assert_eq!(forged.validate(), Err(LifecycleError::InvalidVersion));
}
