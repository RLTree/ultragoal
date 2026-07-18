#[test]
fn unavailable_successor_catalog_has_no_command_inventory_requirements() {
    assert!(super::super::super::required_commands().is_empty());
    assert!(super::super::super::required_surfaces().is_empty());
    assert!(super::super::super::required_loop_stages().is_empty());
    assert!(super::super::super::required_signal_classes().is_empty());
    assert!(super::super::super::required_dimension_families().is_empty());
}
