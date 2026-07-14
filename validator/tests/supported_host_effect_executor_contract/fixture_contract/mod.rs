include!("activation_and_bounds.rs");

include!("recovery.rs");

include!("terminal_states.rs");

include!("refusals_and_claim_ceiling.rs");

#[test]
fn fixture_contract_reconciles_host_execution_and_recovery_limits() {
    let cases = cases();
    assert_activation_and_bounds(&cases);
    assert_recovery_contract(&cases);
    assert_terminal_state_matrix(&cases);
    assert_refusals_and_claim_ceiling(&cases);
}
