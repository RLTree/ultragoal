use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};

#[test]
fn missing_internal_and_relative_json_pointer_targets_fail_without_echo() {
    let repo = TestRepo::new("missing-json-pointers");
    repo.write(
        "schemas/internal.schema.json",
        br##"{"$defs":{"present":{"type":"string"}},"$ref":"#/$defs/not-present"}"##,
    );
    repo.write(
        "schemas/target.schema.json",
        br#"{"$defs":{"present":{"type":"string"}}}"#,
    );
    repo.write(
        "schemas/relative.schema.json",
        br##"{"$ref":"target.schema.json#/$defs/also-not-present"}"##,
    );
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .filter(|finding| finding.code == "invalid_json_reference_source")
            .count()
            >= 2
    );
    let output = String::from_utf8(catalog.to_canonical_json().unwrap()).unwrap();
    assert!(!output.contains("not-present"));
    assert!(!output.contains("also-not-present"));
}
