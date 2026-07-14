use super::fixture::*;
use crate::context::EffectClass;
use crate::inventory::FindingSeverity as InventorySeverity;
use crate::state::catalog::InventoryPolicy;
use crate::state::engine::derive_bound;
use crate::state::product_state::AuthorityRequirement;
use crate::state::snapshot::InventoryObservation;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn projections_are_versioned_valid_json_byte_stable_and_have_no_raw_ansi() {
    let mut inputs = inputs();
    inputs.inventory_findings.push(InventoryObservation {
        code: "seeded".to_owned(),
        severity: InventorySeverity::Error,
        entry_id: Some("entry".to_owned()),
        relative_path: Some("path".to_owned()),
        cause: "seeded failure".to_owned(),
    });
    let mut spec = spec();
    spec.inventory_policies.push(InventoryPolicy {
        code: "seeded".to_owned(),
        scope_surface: "runtime".to_owned(),
        repair: repair(
            "repair-seeded",
            AuthorityRequirement::None,
            EffectClass::Read,
        ),
        ceiling_reductions: reduction(&["runtime"]),
    });
    spec.actions
        .push(command("repair-seeded-action", "repair-seeded", 1));
    let state = derive_bound(inputs, &catalog(spec)).unwrap();
    for projection in [
        state.inspect_json().unwrap(),
        state.diagnose_json().unwrap(),
        state.next_json().unwrap(),
    ] {
        assert_eq!(projection, projection.clone());
        assert!(!projection.contains(&0x1b));
        let value: serde_json::Value = serde_json::from_slice(&projection).unwrap();
        assert!(value["schema_version"].as_str().unwrap().ends_with("-v1"));
    }
    assert_eq!(state.inspect_json().unwrap(), state.inspect_json().unwrap());
    assert_eq!(
        state.diagnose_json().unwrap(),
        state.diagnose_json().unwrap()
    );
    assert_eq!(state.next_json().unwrap(), state.next_json().unwrap());
}

#[test]
fn tampered_receipt_and_view_files_are_not_state_inputs_and_queries_write_nothing() {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-state-pure-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let receipt = root.join("receipt.json");
    let view = root.join("current-state.json");
    fs::write(&receipt, br#"{"claim":"complete"}"#).unwrap();
    fs::write(&view, br#"{"status":"ready"}"#).unwrap();
    let before = directory_bytes(&root);
    let first = derive_bound(inputs(), &catalog(spec())).unwrap();
    let _ = first.inspect_json().unwrap();
    let _ = first.next_json().unwrap();
    let _ = first.diagnose_json().unwrap();
    assert_eq!(
        directory_bytes(&root),
        before,
        "queries changed fixture bytes"
    );

    fs::write(&receipt, br#"{"claim":"tampered"}"#).unwrap();
    fs::write(&view, br#"{"status":"tampered"}"#).unwrap();
    let second = derive_bound(inputs(), &catalog(spec())).unwrap();
    assert_eq!(first.state_id(), second.state_id());
    assert_eq!(first.claim_ceilings(), second.claim_ceilings());
    fs::remove_dir_all(root).unwrap();
}

fn directory_bytes(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(path).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
