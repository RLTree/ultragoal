use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};

#[test]
fn static_overlimit_legacy_scan_is_a_causal_error_finding() {
    let repo = TestRepo::new("legacy-overlimit");
    repo.write("notes/oversized.txt", &vec![b'x'; 2 * 1024 * 1024 + 1]);
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "legacy_model_scan_rejected"
            && finding.relative_path.as_deref() == Some("notes/oversized.txt")
    }));
}
