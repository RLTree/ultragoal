#[test]
fn target_identity_substitution_placeholder_compiles_with_lifecycle_target_module() {
    let fixture = Fixture::new('a');
    let object =
        HostObjectIdentity::from_metadata(&fs::symlink_metadata(&fixture.project).unwrap())
            .unwrap();
    assert!(ObservedTargetIdentity::new(&fixture.scope, 8, object).is_ok());
}
