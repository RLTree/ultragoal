use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::issuer_api_compilation::{check, prepare};
use super::owned_compile_scratch::OwnedCompileScratch;
use super::routine_work::{LocalDirtyTree, PlanRequest, RoutineErrorId};
use super::scenario::{TempRepo, graph};

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
    let owned = OwnedCompileScratch::claim("routine-snapshot-provenance");
    let scratch = owned.path();
    let probes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routine_work_contract/probes");
    prepare(scratch, &probes);

    let control = check(scratch, "routine_snapshot_read_control");
    fs::write(scratch.join("read-control.stderr"), &control.stderr).unwrap();
    assert!(
        control.status.success(),
        "read-only public control failed: {}",
        String::from_utf8_lossy(&control.stderr)
    );

    let constructor = check(scratch, "routine_snapshot_constructor_attack");
    fs::write(scratch.join("constructor.stderr"), &constructor.stderr).unwrap();
    assert!(
        !constructor.status.success(),
        "snapshot constructor compiled"
    );
    assert_specific_failure(&constructor, "E0624", "private");

    let subset = check(scratch, "routine_partial_snapshot_attack");
    fs::write(scratch.join("partial-snapshot.stderr"), &subset.stderr).unwrap();
    assert!(!subset.status.success(), "partial snapshot attack compiled");
    assert_specific_failure(&subset, "E0451", "private");

    let deserialize = check(scratch, "routine_snapshot_deserialize_attack");
    fs::write(scratch.join("deserialize.stderr"), &deserialize.stderr).unwrap();
    assert!(
        !deserialize.status.success(),
        "snapshot deserialize compiled"
    );
    assert_specific_failure(&deserialize, "E0277", "Deserialize");

    let evidence = check(scratch, "routine_evidence_reconstruction_attack");
    fs::write(scratch.join("evidence.stderr"), &evidence.stderr).unwrap();
    assert!(
        !evidence.status.success(),
        "opaque evidence reconstruction compiled"
    );
    assert_specific_failure(&evidence, "E0451", "private");
}

#[test]
fn concurrent_external_consumers_use_disjoint_owned_scratch() {
    let owned = OwnedCompileScratch::claim("routine-snapshot-provenance-concurrency");
    let scratch = owned.path().join("scratch");
    let tmp = owned.path().join("tmp");
    fs::create_dir(&scratch).unwrap();
    fs::create_dir(&tmp).unwrap();
    let sentinel = scratch.join("parent-owned-sentinel");
    fs::write(&sentinel, b"must survive consumer cleanup\n").unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|_| {
            Command::new(&executable)
                .args([
                    "provenance::external_consumer_can_inspect_but_cannot_construct_or_deserialize_snapshot",
                    "--exact",
                ])
                .env("CODEX_WORKTREE_SCRATCH", &scratch)
                .env("CODEX_WORKTREE_TMP", &tmp)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"must survive consumer cleanup\n"
    );
    fs::remove_file(sentinel).unwrap();
    assert_eq!(fs::read_dir(scratch).unwrap().count(), 0);
    assert_eq!(fs::read_dir(tmp).unwrap().count(), 0);
}

fn assert_specific_failure(output: &Output, code: &str, reason: &str) {
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostic.contains(code) && diagnostic.contains(reason),
        "unexpected consumer diagnostic: {diagnostic}"
    );
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
