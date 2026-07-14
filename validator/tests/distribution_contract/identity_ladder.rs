use crate::distribution::{
    DistributionErrorId as ErrorId, IdentitySurface, PackageIdentity, SourceIdentity,
    SurfaceIdentity, verify_identity_ladder, verify_surface_chain,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID};
use serde::Serialize;

const INVENTORY: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const ARCHIVE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OBSERVATION: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn package() -> PackageIdentity {
    PackageIdentity::new(
        SourceIdentity::new(
            CONTEXT_ID.into(),
            CANDIDATE_ID.into(),
            "harness-ultragoal".into(),
            "0.0.12".into(),
            INVENTORY.into(),
            INVENTORY.into(),
        )
        .unwrap(),
        TREE.into(),
        ARCHIVE.into(),
    )
    .unwrap()
}

fn rows() -> Vec<SurfaceIdentity> {
    IdentitySurface::ALL
        .into_iter()
        .map(|surface| {
            SurfaceIdentity::new(
                package(),
                surface,
                OBSERVATION.into(),
                matches!(surface, IdentitySurface::Installed | IdentitySurface::Cache)
                    .then(|| TREE.into()),
            )
            .unwrap()
        })
        .collect()
}

#[derive(Serialize)]
struct Ladder<'a> {
    schema: &'static str,
    surfaces: &'a [SurfaceIdentity],
}

#[test]
fn identity_ladder_keeps_source_package_and_surface_payloads_distinct() {
    let rows = rows();
    let bytes = serde_json::to_vec(&Ladder {
        schema: "harness-ultragoal.distribution-identity-ladder.v1",
        surfaces: &rows,
    })
    .unwrap();
    let verified = verify_identity_ladder(&bytes).unwrap();
    assert_eq!(verified.len(), IdentitySurface::ALL.len());
    assert_eq!(
        verified[0].package().source().accepted_inventory_sha256(),
        INVENTORY
    );
    assert_eq!(verified[0].package().source().catalog_id(), INVENTORY);
    assert_eq!(verified[0].package().tree_sha256(), TREE);
    assert_eq!(verified[0].package().archive_sha256(), ARCHIVE);
    assert_eq!(verified[0].observation_sha256(), OBSERVATION);
    assert_ne!(INVENTORY, TREE);
    assert_ne!(TREE, ARCHIVE);
    assert_ne!(ARCHIVE, OBSERVATION);
}

#[test]
fn missing_reordered_duplicate_and_tree_substitution_ladders_fail_closed() {
    let mut reordered = rows();
    reordered.swap(0, 1);
    assert_eq!(
        verify_surface_chain(&reordered).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
    );
    let mut missing = rows();
    missing.pop();
    assert_eq!(
        verify_surface_chain(&missing).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
    );
    assert_eq!(
        SurfaceIdentity::new(
            package(),
            IdentitySurface::Cache,
            OBSERVATION.into(),
            Some(INVENTORY.into()),
        )
        .unwrap_err()
        .id(),
        ErrorId::ProvenanceMismatch
    );
}

#[test]
fn unknown_identity_fields_are_rejected() {
    let rows = rows();
    let mut value = serde_json::to_value(Ladder {
        schema: "harness-ultragoal.distribution-identity-ladder.v1",
        surfaces: &rows,
    })
    .unwrap();
    value["surfaces"][0]["unknown"] = serde_json::json!(true);
    assert_eq!(
        verify_identity_ladder(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::InvalidSpec
    );
}
