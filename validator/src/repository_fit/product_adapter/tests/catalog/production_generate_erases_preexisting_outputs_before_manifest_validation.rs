use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn production_generate_erases_preexisting_outputs_before_manifest_validation() {
    use std::os::unix::fs::symlink;

    let fixture = SourceFixture::new("generate-manifest-early-failure");
    fixture.write("A.md", b"trusted");
    let real_manifest = fixture.root.join("real-manifest.json");
    fs::write(&real_manifest, manifest(&["templates/A.md"])).unwrap();
    symlink(&real_manifest, fixture.manifest_path()).unwrap();
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[cfg(unix)]
#[test]
pub(crate) fn production_generate_erases_preexisting_outputs_on_real_source_symlink_failure() {
    use std::os::unix::fs::symlink;

    let fixture = SourceFixture::new("generate-source-symlink-failure");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fs::write(fixture.root.join("real-source"), b"trusted").unwrap();
    symlink("../real-source", fixture.templates.join("A.md")).unwrap();
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_erases_preexisting_outputs_on_parse_and_initial_parity_failures()
{
    let malformed = SourceFixture::new("generate-parse-failure");
    malformed.write("A.md", b"trusted");
    malformed.write_manifest(b"not-json");
    malformed.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &malformed.manifest_path(),
            &malformed.templates,
            &malformed.output,
        )
        .is_err()
    );
    malformed.assert_authority_outputs_absent();

    let parity = SourceFixture::new("generate-initial-parity-failure");
    parity.write("A.md", b"trusted");
    parity.write("B.md", b"extra");
    parity.write_manifest(&manifest(&["templates/A.md"]));
    parity.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(&parity.manifest_path(), &parity.templates, &parity.output,)
            .is_err()
    );
    parity.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_erases_outputs_on_final_reenumeration_failure() {
    let fixture = SourceFixture::new("generate-final-parity-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || fixture.write("B.md", b"extra"),
            || {},
            || {},
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_catches_hook_panic_and_erases_outputs() {
    let fixture = SourceFixture::new("generate-hook-panic");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    let result = production_sources::generate_with_hooks(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
        || panic!("controlled build-source hook panic"),
        || {},
        || {},
    );
    assert_eq!(result.unwrap_err(), "template source staging panicked");
    fixture.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_erases_outputs_on_staged_manifest_write_failure() {
    let fixture = SourceFixture::new("generate-manifest-write-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || {},
            || fs::create_dir(fixture.staging().join("manifest.json")).unwrap(),
            || {},
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_erases_outputs_on_catalog_emission_failure() {
    let fixture = SourceFixture::new("generate-catalog-emission-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || {},
            || {},
            || fs::create_dir(fixture.catalog()).unwrap(),
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
pub(crate) fn production_generate_success_replaces_stale_outputs_with_exact_new_outputs() {
    let fixture = SourceFixture::new("generate-success-exact-outputs");
    fixture.write("A.md", b"trusted");
    let manifest_bytes = manifest(&["templates/A.md"]);
    fixture.write_manifest(&manifest_bytes);
    fixture.prepopulate_authority_outputs();
    production_sources::generate(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
    )
    .unwrap();
    assert_eq!(
        fs::read(fixture.staging().join("0000.bin")).unwrap(),
        b"trusted"
    );
    assert_eq!(
        fs::read(fixture.staging().join("manifest.json")).unwrap(),
        manifest_bytes
    );
    let mut staged_names = fs::read_dir(fixture.staging())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    staged_names.sort();
    assert_eq!(staged_names, ["0000.bin", "manifest.json"]);
    let catalog = fs::read_to_string(fixture.catalog()).unwrap();
    assert!(catalog.starts_with("const MANIFEST_BYTES"));
    assert!(!catalog.contains("stale generated catalog"));
}
