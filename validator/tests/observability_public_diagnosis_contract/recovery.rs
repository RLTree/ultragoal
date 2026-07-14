use super::scenario::*;
use std::fs::{self, OpenOptions};
use std::io::Write;

#[test]
fn corruption_is_read_only_until_explicit_truncated_tail_recovery() {
    let repository = Repository::new("recovery", true, false);
    let binding = binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let mut retained = event(&binding, "retained-failure", 1, "check.run", "fail");
    retained.add_finding_ref(&finding.finding_id).unwrap();
    assert!(store.append(&retained).unwrap());
    OpenOptions::new()
        .append(true)
        .open(repository.store_path())
        .unwrap()
        .write_all(b"{\"row_version\":\"SemanticEventRow-v1\"")
        .unwrap();
    let corrupt_bytes = fs::read(repository.store_path()).unwrap();
    let (_, diagnostic) =
        assert_diagnostic_zero_write(&repository, &["--json", "observe", "query"], 4);
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    let (_, diagnosis) = assert_payload_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["read_failure"]["stage"], "query");
    assert_eq!(
        diagnosis["observability"]["read_failure"]["class"],
        "corruption"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "corruption"
    );
    assert_eq!(fs::read(repository.store_path()).unwrap(), corrupt_bytes);
    let removed = store.recover_truncated_tail().unwrap();
    assert!(removed > 0);
    assert_eq!(store.recover_truncated_tail().unwrap(), 0);
    let (_, recovered) = assert_payload_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &[],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(recovered["events"][0]["event_id"], "retained-failure");
}
