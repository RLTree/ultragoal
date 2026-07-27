use super::*;

#[test]
fn every_store_path_times_out_under_identity_contention_without_writing() {
    let fixture = TestStore::seeded("all-paths");
    let before = fixture.store_bytes();

    assert_identity_timeout(&fixture.store, || fixture.store.query(&fixture.query()));
    assert_identity_timeout(&fixture.store, || {
        fixture.store.explain(&fixture.query(), "seed")
    });
    assert_identity_timeout(&fixture.store, || {
        fixture.store.append(&event("blocked", 2, 2, "fail"))
    });
    assert_identity_timeout(&fixture.store, || fixture.store.clear());
    assert_identity_timeout(&fixture.store, || fixture.store.recover_truncated_tail());

    let mut adapter = NoEffectAdapter { calls: 0 };
    assert_identity_timeout(&fixture.store, || {
        fixture.store.export_explicit(ExplicitExportRequest {
            query: &fixture.query(),
            configured: true,
            consent_granted: true,
            timeout: Duration::from_secs(2),
            adapter: Some(&mut adapter),
        })
    });
    assert_eq!(adapter.calls, 0, "contended export reached its adapter");
    assert_eq!(fixture.store_bytes(), before, "contended path wrote bytes");
    assert_eq!(fixture.store.query(&fixture.query()).unwrap().len(), 1);
    fixture.remove();
}
