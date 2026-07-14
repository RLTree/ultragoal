#[test]
fn lifecycle_candidate_has_no_public_construction_or_legacy_execution_route() {
    let cases = cases();
    assert_eq!(cases.public_api_contract.module_visibility, "crate-private");
    assert_eq!(cases.public_api_contract.public_constructors, 0);
    assert!(!cases.public_api_contract.cloneable_permit_or_handoff);
    assert!(!cases.public_api_contract.deserializable_permit_or_handoff);
    assert!(
        !cases
            .public_api_contract
            .production_descriptor_executor_implemented
    );

    let lifecycle = source("validator/src/distribution/host_effect/lifecycle.rs");
    assert!(!lifecycle.contains("pub mod "));
    for path in [
        "validator/src/distribution/host_effect/lifecycle.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/coordinator.rs",
        "validator/src/distribution/host_effect/lifecycle/recovery.rs",
    ] {
        let body = source(path);
        assert!(
            !body.contains("pub fn new("),
            "public constructor in {path}"
        );
        assert!(
            !body.contains("fn into_parts("),
            "owned authority split in {path}"
        );
        for forbidden in [
            "HostAuthorization",
            "execute_authorized",
            "HostExecutor",
            "PreparedExternalHostEffect",
            ".into_plan(",
            "Command::new",
            "std::process",
        ] {
            assert!(
                !body.contains(forbidden),
                "legacy/effect route {forbidden} in {path}"
            );
        }
    }
    assert!(
        source("validator/src/distribution/host_effect/lifecycle/coordinator.rs")
            .contains("fn with_retained_authority")
    );
}
