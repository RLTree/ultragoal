use crate::context::{BuildRequest, LiveContext};
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};
use std::fs;

#[test]
fn adopted_contract_digest_is_root_bound_and_tamper_evident() {
    let repo = TestRepo::new("contract-tamper");
    let tool_inventory = repo.root.join(
        "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CUSTOM_TOOL_INVENTORY.json",
    );
    fs::write(
        &tool_inventory,
        br#"{"contract_id":"harness-ultragoal-successor-contract-v2","tool_count":0,"tools":[]}"#,
    )
    .unwrap();
    let manifest = repo
        .root
        .join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256");
    let manifest_text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        manifest_text.replace(
            "03073c8ee853d02be0776443cf39828d0630d3e1389d69be7817b201fc971fb7",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    )
    .unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("root-bound digest"));

    let unbound = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&unbound).build().unwrap_err();
    assert!(error.to_string().contains("did not bind"));
}

fn route_header_error(field: &str) -> String {
    let repo = TestRepo::new("route-header-canary");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value[field] = serde_json::json!("SECRET_CANARY\n\u{1b}[31m");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context)
        .build()
        .unwrap_err()
        .to_string()
}

#[test]
fn routing_header_errors_do_not_echo_untrusted_values() {
    for field in ["schema_version", "contract_id"] {
        let error = route_header_error(field);
        assert!(!error.contains("SECRET_CANARY"));
        assert!(!error.contains('\n'));
        assert!(!error.contains('\u{1b}'));
    }
}
