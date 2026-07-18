use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn production_generate_reports_cleanup_failure_before_validation() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = SourceFixture::new("generate-cleanup-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(b"not-json");
    fixture.prepopulate_authority_outputs();
    fs::set_permissions(&fixture.output, fs::Permissions::from_mode(0o555)).unwrap();
    let result = production_sources::generate(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
    );
    fs::set_permissions(&fixture.output, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        result.unwrap_err(),
        "authority-bearing output cleanup failed"
    );
    assert!(fixture.catalog().exists() || fixture.staging().exists());
}
