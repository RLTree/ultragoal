#[test]
fn darwin_contract_is_zero_reservation_zero_spawn_zero_effect_and_repeatable() {
    let darwin = cases().darwin;
    assert_eq!(darwin.result, "unsupported-platform");
    assert!(darwin.repeated_attempts >= 2);
    assert_eq!(darwin.expected_clock_samples, 0);
    assert_eq!(darwin.expected_target_observations, 0);
    assert_eq!(darwin.expected_descriptor_adapter_calls, 0);
    assert_eq!(darwin.expected_ledger_reservations, 0);
    assert_eq!(darwin.expected_ledger_transitions, 0);
    assert_eq!(darwin.expected_process_spawns, 0);
    assert_eq!(darwin.expected_host_effects, 0);
    assert_eq!(darwin.expected_plan_releases, 0);
    assert!(darwin.proof.contains("recursive"));

    let coordinator = source("validator/src/distribution/host_effect/lifecycle/coordinator.rs");
    let unsupported = coordinator
        .find("if platform == DescriptorExecutionPlatform::Darwin")
        .unwrap();
    for later in [
        "adapter.descriptor_capability()",
        "clock.sample()",
        "target.acquire(",
        ".ledger.head()",
        ".issue(binding)",
        ".reserve(reservation)",
        "custody.commit_release()",
    ] {
        assert!(unsupported < coordinator[unsupported..].find(later).unwrap() + unsupported);
    }
}
