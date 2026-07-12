use crate::orchestration::*;
use crate::support::*;

fn permit_read(policy: &mut ScopePolicy, lease: &LeaseSpec) {
    policy.allowed_read_paths.extend(lease.read_paths.clone());
}

fn assert_invalid_lease(lease: &LeaseSpec, policy: &ScopePolicy) {
    let error = lease.validate(policy).unwrap_err();
    assert_eq!(error, OrchestrationError::InvalidLease);
    assert_eq!(error.code(), "HUL-ORCH-011");
    assert_eq!(error.to_string(), "HUL-ORCH-011: lease contract is invalid");
}

#[test]
fn disjoint_read_and_owned_paths_remain_valid() {
    let lease = lease();
    lease.validate(&policy()).unwrap();

    let package = package("node-a", &[], "node_a");
    package.validate().unwrap();
}

#[test]
fn exact_ancestor_and_child_read_write_overlap_fail_closed() {
    let cases = [
        (
            "validator/src/orchestration/node_a.rs",
            "validator/src/orchestration/node_a.rs",
        ),
        (
            "validator/src/orchestration",
            "validator/src/orchestration/node_a.rs",
        ),
        (
            "validator/src/orchestration/node_a.rs",
            "validator/src/orchestration",
        ),
    ];
    for (read, owned) in cases {
        let mut lease = lease();
        lease.read_paths = [path(read)].into();
        lease.owned_scope.paths = [path(owned)].into();
        let mut expanded = policy();
        permit_read(&mut expanded, &lease);
        expanded.allowed_paths.insert(path(owned));
        assert_invalid_lease(&lease, &expanded);
    }
}

#[test]
fn ascii_case_file_and_directory_aliases_fail_closed_without_echo() {
    for alias in [
        "VALIDATOR/SRC/ORCHESTRATION/NODE_A.RS",
        "VALIDATOR/SRC/ORCHESTRATION",
    ] {
        let mut lease = lease();
        lease.read_paths = [path(alias)].into();
        let mut expanded = policy();
        permit_read(&mut expanded, &lease);
        let error = lease.validate(&expanded).unwrap_err();
        assert_eq!(error, OrchestrationError::InvalidLease);
        assert!(!error.to_string().contains("VALIDATOR"));
    }
}

#[test]
fn generated_output_and_fixture_aliases_cannot_be_read_authority() {
    for alias in [
        "GENERATED/ORCHESTRATION/NODE_A.JSON",
        "VALIDATOR/TESTS/ORCHESTRATION_CONTRACT/NODE_A.JSON",
    ] {
        let mut lease = lease();
        lease.read_paths = [path(alias)].into();
        let mut expanded = policy();
        permit_read(&mut expanded, &lease);
        assert_invalid_lease(&lease, &expanded);
    }
}

#[test]
fn mutation_after_lease_construction_revalidates_the_invariant() {
    let mut lease = lease();
    let mut expanded = policy();
    lease.validate(&expanded).unwrap();

    lease.read_paths.insert(path("VALIDATOR/SRC/ORCHESTRATION"));
    permit_read(&mut expanded, &lease);
    assert_invalid_lease(&lease, &expanded);
}

#[test]
fn node008_owned_to_read_case_alias_false_pass_is_rejected() {
    let mut lease = lease();
    lease.read_paths = [path("VALIDATOR/SRC/ORCHESTRATION")].into();
    let mut expanded = policy();
    permit_read(&mut expanded, &lease);
    assert_invalid_lease(&lease, &expanded);

    let mut package = package("node-a", &[], "node_a");
    package.read_paths = lease.read_paths.clone();
    let mut result = result_for(&lease, &package);
    result.artifacts[0].path = "VALIDATOR/SRC/ORCHESTRATION/NODE_A.RS".to_owned();
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidLease
    );
}

#[test]
fn ambiguous_work_package_is_rejected_before_graph_derivation() {
    let mut package = package("node-a", &[], "node_a");
    package.read_paths = [path("VALIDATOR/SRC/ORCHESTRATION")].into();
    assert_eq!(
        package.validate().unwrap_err(),
        OrchestrationError::InvalidTransition
    );
    assert!(WorkGraph::derive(vec![package]).is_err());
}

#[test]
fn rejected_alias_lease_appends_no_event() {
    let (mut engine, _) = engine();
    let mut lease = lease();
    lease.read_paths.insert(path("VALIDATOR/SRC/ORCHESTRATION"));
    let before = engine.event_log();
    assert_eq!(
        engine.grant_lease(1, lease).unwrap_err(),
        OrchestrationError::InvalidLease
    );
    assert_eq!(engine.event_log(), before);
}
