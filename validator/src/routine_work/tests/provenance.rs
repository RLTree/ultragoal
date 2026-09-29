use super::routine_work::{LocalDirtyTree, PlanRequest, RoutineErrorId};
use super::scenario::{TempRepo, graph};

#[test]
fn structurally_valid_subset_without_capture_provenance_is_rejected() {
    let mut repo = TempRepo::new("subset-provenance");
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
    repo.teardown_after_assertions();
}
