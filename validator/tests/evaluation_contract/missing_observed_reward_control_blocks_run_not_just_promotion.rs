#[test]
fn missing_observed_reward_control_blocks_run_not_just_promotion() {
    let spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 2);
    let mut incomplete = controls();
    incomplete.remove(&PerturbationControl::ReceiptProduction);
    executor.rows.insert(
        "core".to_owned(),
        CapturedTaskObservation::captured(CapturedTaskObservationRecord {
            task_id: "core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            outcome: BehaviorOutcome::Failed,
            causal_code: "causal-product-failure".to_owned(),
            score_earned: 2,
            score_possible: 10,
            work_units: 1,
            artifact: BoundInput::regular("artifacts/core.json", sha('d'), 32),
            replay_artifact_digest_sha256: sha('d'),
            producer_id: "executor-core".to_owned(),
            observer_id: "observer-core".to_owned(),
            independent_grader_id: "grader-core".to_owned(),
            independent_score_earned: 2,
            independent_score_possible: 10,
            passed_perturbations: incomplete,
        }),
    );
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-observation-refused"
    );
}

#[test]
fn audit_and_reconciliation_are_zero_write() {
    let root = temp_root("zero-write");
    fs::write(root.join("sentinel"), b"preserve").unwrap();
    let before = tree(&root);
    let spec = spec('1');
    let _ = spec.audit(&sha('c'), &sha('1'));
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = review_authority(&baseline, &candidate);
    let review = review(&baseline, &candidate, &mut authority);
    let _ = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(tree(&root), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_callers_cannot_mint_a_review_with_an_independence_boolean() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/evaluation/promotion/issuance.rs"),
    )
    .unwrap();
    let issuance = source
        .split_once("fn issue(")
        .map(|(_, remainder)| remainder)
        .expect("sealed PromotionReview issuance remains present");
    assert!(issuance.contains("authority: &mut PromotionReviewAuthority"));
    assert!(!issuance.contains("pub fn issue"));
    assert!(!issuance.contains("pub(crate) fn issue"));
    assert!(!issuance.contains("independent: bool"));
    assert!(!source.contains("trait PromotionReview"));
}

#[test]
fn fixture_catalog_names_every_false_pass_family() {
    let paired: Value = serde_json::from_str(include_str!(
        "../../../fixtures/evaluation-engine/paired-valid.json"
    ))
    .unwrap();
    let red: Value = serde_json::from_str(include_str!(
        "../../../fixtures/evaluation-engine/red-cases.json"
    ))
    .unwrap();
    assert_eq!(paired["schema_version"], "EvaluationEngineFixture-v1");
    let cases = red["cases"].as_array().unwrap();
    assert!(cases.len() >= 24);
    for expected in [
        "metric-gain-without-behavior",
        "verbosity-reward",
        "proof-artifact-reward",
        "receipt-production-reward",
        "test-manipulation-reward",
        "score-only-promotion",
        "caller-asserted-independent-boolean",
        "same-review-source-principal",
        "same-review-session",
        "stale-review-binding",
        "substituted-journey-evidence",
        "review-attestation-replay",
    ] {
        assert!(cases.iter().any(|case| case == expected));
    }
}

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-evaluation-043-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    walk(root, root, &mut out);
    out
}
