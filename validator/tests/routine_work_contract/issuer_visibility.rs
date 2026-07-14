use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::provenance_compile::{compile_public_consumer, document_public_surface};

#[test]
fn sealed_issuer_and_grant_entrypoints_are_not_externally_callable() {
    let scratch = PathBuf::from("/tmp/hul-routine-issuer-visibility-001");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let dependencies = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let probes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routine_work_contract/probes");

    let control = compile_public_consumer(
        &probes.join("routine_public_api_control.rs"),
        "routine_public_api_control",
        &scratch,
        &dependencies,
    );
    assert!(control.status.success(), "{}", diagnostic(&control));

    for (probe, code) in [
        ("production_issuer_consumer.rs", "E0603"),
        ("production_grant_consumer.rs", "E0432"),
        ("production_grant_entrypoint_consumer.rs", "E0432"),
        ("production_private_module_consumer.rs", "E0603"),
        ("production_private_grant_consumer.rs", "E0603"),
    ] {
        let output = compile_public_consumer(
            &probes.join(probe),
            probe.trim_end_matches(".rs"),
            &scratch,
            &dependencies,
        );
        assert_private_failure(&output, code, probe);
    }

    let docs = document_public_surface(&scratch, &dependencies);
    let inventory = public_inventory(&docs);
    let digest = format!("{:x}", Sha256::digest(&inventory));
    assert_eq!(
        digest,
        "8681aa05bed5c6b42f59f01dfadd68c47ea4049befccd9bd948ec4b40e7e4629"
    );

    let mut source = OpenOptions::new()
        .append(true)
        .open(scratch.join("public_surface.rs"))
        .unwrap();
    source
        .write_all(b"\npub fn test_grant_entrypoint() {}\n")
        .unwrap();
    let mutated = public_inventory(&document_public_surface(&scratch, &dependencies));
    assert_ne!(Sha256::digest(&mutated), Sha256::digest(&inventory));
}

fn public_inventory(docs: &Path) -> Vec<u8> {
    fs::read(docs.join("public_surface/sidebar-items.js")).unwrap()
}

fn assert_private_failure(output: &Output, code: &str, probe: &str) {
    let text = diagnostic(output);
    assert!(!output.status.success(), "{probe} became callable");
    assert!(text.contains(code), "unexpected {probe} failure: {text}");
    for incidental in ["can't find crate", "file not found", "couldn't read"] {
        assert!(!text.contains(incidental), "incidental {probe}: {text}");
    }
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
