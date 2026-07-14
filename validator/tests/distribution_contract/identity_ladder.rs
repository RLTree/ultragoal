use crate::distribution::{DistributionErrorId as ErrorId, verify_identity_ladder};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID};
use serde_json::{Value, json};

const INVENTORY: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const ARCHIVE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OBSERVATION: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const BINDING: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn forged_ladder() -> Value {
    let package = json!({
        "source": {
            "context_id": CONTEXT_ID,
            "candidate_id": CANDIDATE_ID,
            "plugin_id": "harness-ultragoal",
            "version": "0.0.12",
            "catalog_id": INVENTORY,
            "accepted_inventory_sha256": INVENTORY
        },
        "tree_sha256": TREE,
        "archive_sha256": ARCHIVE
    });
    let surfaces = [
        "package",
        "installed",
        "cache",
        "marketplace",
        "app-registry",
        "discovery",
        "runtime",
    ]
    .into_iter()
    .map(|surface| {
        let mut row = json!({
            "package": package.clone(),
            "surface": surface,
            "observation_sha256": OBSERVATION,
            "journey_binding_sha256": BINDING
        });
        if matches!(surface, "installed" | "cache") {
            row["observed_tree_sha256"] = json!(TREE);
        }
        row
    })
    .collect::<Vec<_>>();
    json!({
        "schema": "harness-ultragoal.distribution-identity-ladder.v1",
        "surfaces": surfaces
    })
}

#[test]
fn caller_forged_package_and_app_registry_rows_never_enter_the_canonical_verifier() {
    let bytes = serde_json::to_vec(&forged_ladder()).unwrap();
    assert_eq!(
        verify_identity_ladder(&bytes).unwrap_err().id(),
        ErrorId::ProvenanceMismatch
    );
}

#[test]
fn serialized_shape_changes_cannot_upgrade_untrusted_rows_to_authority() {
    let mut forged = forged_ladder();
    forged["surfaces"][0]["unknown"] = json!(true);
    assert_eq!(
        verify_identity_ladder(&serde_json::to_vec(&forged).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
}
