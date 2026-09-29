#[test]
fn sibling_custody_route_has_no_prepare_or_forged_publication_surface() {
    let owner = include_str!("../ledger/owner.rs");
    let request = include_str!("../ledger/owner_request.rs");
    let transaction = include_str!("../ledger/owner_transaction.rs");
    assert!(!owner.contains("pub(super) fn prepare"));
    assert!(!request.contains("pub(crate) fn prepare"));
    assert!(!transaction.contains("ExecutionPublication"));
    assert!(!transaction.contains("FnOnce"));
}
