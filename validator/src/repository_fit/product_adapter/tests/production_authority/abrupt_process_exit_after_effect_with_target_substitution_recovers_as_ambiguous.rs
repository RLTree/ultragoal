use super::*;

#[test]
pub(crate) fn abrupt_process_exit_after_effect_with_target_substitution_recovers_as_ambiguous() {
    let fixture = Fixture::new("process-crash-after-effect-substitution");
    let _intent = run_authority_crash(
        &fixture,
        "crash-after",
        "crash-after-substitution-owner",
        88,
    );
    let attacked = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
    fs::write(&attacked, b"attacker substituted after the owner exited\n").unwrap();

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-after-substitution-owner",
    );
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(
        fs::read(&attacked).unwrap(),
        b"attacker substituted after the owner exited\n"
    );
    assert_eq!(recovered["effect"], "none");
    assert_eq!(recovered["ledger_state"], "ambiguous");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
pub(crate) fn abrupt_process_exit_after_effect_start_with_partial_postimage_recovers_as_ambiguous()
{
    let fixture = Fixture::new("process-crash-partial-effect");
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-after-start", "crash-partial-owner", 89);
    assert_eq!(snapshot(&fixture.root), before);
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let partial = snapshot(&fixture.root);

    let recovered = run_authority_scenario(&fixture, "recover-expired", "crash-partial-owner");
    assert_eq!(recovered["status"], "ambiguous");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "ambiguous");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(snapshot(&fixture.root), partial);

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"ambiguous\"").count(), 1);
    assert!(!terminal.contains("committed"));
}

#[test]
pub(crate) fn abrupt_process_exit_after_effect_start_with_exact_existing_ancestor_preimage_is_interrupted()
 {
    let fixture = Fixture::new("process-crash-existing-ancestor-preimage");
    create_existing_codex_ancestor(&fixture);
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(
        &fixture,
        "crash-after-start",
        "crash-existing-ancestor-preimage-owner",
        89,
    );
    assert_eq!(snapshot(&fixture.root), before);

    let recovered = run_authority_scenario(
        &fixture,
        "recover-expired",
        "crash-existing-ancestor-preimage-owner",
    );
    assert_eq!(recovered["status"], "interrupted");
    assert_eq!(recovered["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(recovered["ledger_state"], "interrupted");
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn recovery_intent_parse_and_inspect_are_bounded_canonical_and_zero_write() {
    let fixture = Fixture::new("recovery-intent-parse");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let before_store = fixture.store_names();

    let intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let bytes = intent.to_machine_bytes();
    let parsed = parse_recovery_intent(&bytes).unwrap();
    assert_eq!(parsed.to_machine_bytes(), bytes);
    assert!(!String::from_utf8_lossy(&bytes).contains(fixture.root.to_str().unwrap()));
    assert!(bytes.len() < 64 * 1024);

    let mut mutated = bytes.clone();
    mutated[0] ^= 1;
    assert_eq!(
        parse_recovery_intent(&mutated).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );
    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert_eq!(
        parse_recovery_intent(&noncanonical).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );
    let oversize = vec![b'x'; 64 * 1024 + 1];
    assert_eq!(
        parse_recovery_intent(&oversize).unwrap_err().id(),
        AdapterErrorId::ApplyPermitInvalid
    );

    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
    assert_eq!(fixture.store_names(), before_store);
}

#[test]
pub(crate) fn wrong_nonce_and_stale_canonical_intent_refuse_without_terminal_mutation() {
    let fixture = Fixture::new("recovery-intent-binding");
    let intent_path = run_authority_crash(
        &fixture,
        "crash-before",
        "recovery-intent-binding-owner",
        86,
    );
    let intent_bytes = fs::read(intent_path).unwrap();
    let before_target = snapshot(&fixture.root);
    let ledger_path = fixture.store.root.join("authority-ledger.json");
    let before_ledger = fs::read(&ledger_path).unwrap();
    let context = fixture.context();

    let wrong_nonce = recover_prepared_apply(
        &context,
        &intent_bytes,
        &TestClock::new([73]),
        &fixture.store,
        nonce("recovery-intent-wrong-owner"),
    );
    assert_eq!(wrong_nonce.status(), "refused");
    assert_eq!(
        wrong_nonce.error_id(),
        Some(AdapterErrorId::ApplyPermitReplayed)
    );
    assert_eq!(fs::read(&ledger_path).unwrap(), before_ledger);
    assert_eq!(snapshot(&fixture.root), before_target);

    let other = Fixture::new("recovery-intent-stale-source");
    let other_context = other.context();
    let stale_intent = prepare_recovery_intent(&other_context, &other.prepared(&other_context))
        .unwrap()
        .to_machine_bytes();
    let stale = recover_prepared_apply(
        &context,
        &stale_intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("recovery-intent-binding-owner"),
    );
    assert_eq!(stale.status(), "refused");
    assert_eq!(stale.error_id(), Some(AdapterErrorId::ApplyPermitReplayed));
    assert_eq!(fs::read(&ledger_path).unwrap(), before_ledger);
    assert_eq!(snapshot(&fixture.root), before_target);

    let recovered =
        run_authority_scenario(&fixture, "recover-expired", "recovery-intent-binding-owner");
    assert_eq!(recovered["status"], "interrupted");
}
