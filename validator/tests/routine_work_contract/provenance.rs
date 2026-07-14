use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::provenance_compile::{compile_consumer, compile_surface};
use super::routine_work::{LocalDirtyTree, PlanRequest, RoutineErrorId};
use super::scenario::{TempRepo, graph};

const SCRATCH: &str = "/tmp/hul-routine-snapshot-provenance-001/consumer-probe";

#[test]
fn structurally_valid_subset_without_capture_provenance_is_rejected() {
    let repo = TempRepo::new("subset-provenance");
    repo.write("docs/guide.md", b"changed guide\n");
    repo.write("release.json", b"{\"changed\":true}\n");
    let context = repo.context("routine");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    assert_eq!(snapshot.changes().len(), 2);

    let full =
        super::routine_work::plan_routine(&context, &graph(), &snapshot, PlanRequest::routine())
            .unwrap();
    assert!(full.check("release").is_some());

    let forged = snapshot.test_forged_subset_with_recomputed_identity();
    assert_eq!(forged.binding(), snapshot.binding());
    assert_eq!(forged.status_sha256(), snapshot.status_sha256());
    assert_eq!(forged.changes().len(), 1);
    assert_ne!(forged.snapshot_id(), snapshot.snapshot_id());
    let error =
        super::routine_work::plan_routine(&context, &graph(), &forged, PlanRequest::routine())
            .unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::InvalidSnapshot);
    assert_eq!(error.cause(), "snapshot-capture-provenance-invalid");
}

#[test]
fn external_consumer_can_inspect_but_cannot_construct_or_deserialize_snapshot() {
    let scratch = PathBuf::from(SCRATCH);
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let dependencies = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let surface = compile_surface(&scratch, &dependencies);
    let probes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routine_work_contract/probes");

    let control = compile_consumer(
        &probes.join("read_only_snapshot_consumer.rs"),
        "routine_snapshot_read_control",
        &scratch,
        &dependencies,
        &surface,
    );
    fs::write(scratch.join("read-control.stderr"), &control.stderr).unwrap();
    assert!(
        control.status.success(),
        "read-only public control failed: {}",
        String::from_utf8_lossy(&control.stderr)
    );

    let constructor = compile_consumer(
        &probes.join("snapshot_constructor_consumer.rs"),
        "routine_snapshot_constructor_attack",
        &scratch,
        &dependencies,
        &surface,
    );
    fs::write(scratch.join("constructor.stderr"), &constructor.stderr).unwrap();
    assert!(
        !constructor.status.success(),
        "snapshot constructor compiled"
    );
    assert_specific_failure(&constructor, "E0624", "private");

    let subset = compile_consumer(
        &probes.join("partial_snapshot_consumer.rs"),
        "routine_partial_snapshot_attack",
        &scratch,
        &dependencies,
        &surface,
    );
    fs::write(scratch.join("partial-snapshot.stderr"), &subset.stderr).unwrap();
    assert!(!subset.status.success(), "partial snapshot attack compiled");
    assert_specific_failure(&subset, "E0451", "private");

    let deserialize = compile_consumer(
        &probes.join("snapshot_deserialize_consumer.rs"),
        "routine_snapshot_deserialize_attack",
        &scratch,
        &dependencies,
        &surface,
    );
    fs::write(scratch.join("deserialize.stderr"), &deserialize.stderr).unwrap();
    assert!(
        !deserialize.status.success(),
        "snapshot deserialize compiled"
    );
    assert_specific_failure(&deserialize, "E0277", "Deserialize");

    let evidence = compile_consumer(
        &probes.join("evidence_reconstruction_consumer.rs"),
        "routine_evidence_reconstruction_attack",
        &scratch,
        &dependencies,
        &surface,
    );
    fs::write(scratch.join("evidence.stderr"), &evidence.stderr).unwrap();
    assert!(
        !evidence.status.success(),
        "opaque evidence reconstruction compiled"
    );
    assert_specific_failure(&evidence, "E0451", "private");
}

fn assert_specific_failure(output: &Output, code: &str, reason: &str) {
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains(code) && diagnostic.contains(reason));
    for incidental in [
        "unresolved import",
        "can't find crate",
        "file not found for module",
        "couldn't read",
    ] {
        assert!(
            !diagnostic.contains(incidental),
            "incidental probe failure: {incidental}: {diagnostic}"
        );
    }
}
