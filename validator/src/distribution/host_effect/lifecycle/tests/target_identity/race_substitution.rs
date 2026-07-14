#[test]
fn target_race_substitution_placeholder_compiles_with_lifecycle_target_module() {
    let fixture = Fixture::new('9');
    assert_eq!(fixture.target.generation(), 7);
}
