use super::plugin_product::product_fitness::{
    ClaimCeiling, DimensionDisposition, DimensionEvidence, EvidenceBinding, FitnessDimension,
    OverallDisposition, ProductFitnessDisposition, ProductFitnessError, TRUTH_LAYERS, TruthLayer,
    source_candidate_ceilings,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const CANDIDATE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "hul-product-fitness-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn disposition(
    root: &Path,
    per_dimension: impl Fn(FitnessDimension) -> DimensionDisposition,
) -> ProductFitnessDisposition {
    fs::write(root.join("evidence.json"), b"same-surface-evidence\n").unwrap();
    let evidence = EvidenceBinding {
        path: "evidence.json".to_owned(),
        sha256: digest(b"same-surface-evidence\n"),
        candidate_id: CANDIDATE.to_owned(),
        same_surface: true,
        current_session: true,
    };
    let dimensions = [
        FitnessDimension::Accessibility,
        FitnessDimension::CognitiveLoad,
        FitnessDimension::RecoveryBurden,
        FitnessDimension::Continuance,
        FitnessDimension::RealUseEvidence,
    ]
    .into_iter()
    .map(|dimension| DimensionEvidence {
        dimension,
        disposition: per_dimension(dimension),
        finding: format!("{dimension:?} has exact evidence"),
        evidence: evidence.clone(),
    })
    .collect::<Vec<_>>();
    let overall = if dimensions
        .iter()
        .any(|item| item.disposition == DimensionDisposition::Fail)
    {
        OverallDisposition::Fail
    } else if dimensions
        .iter()
        .any(|item| item.disposition == DimensionDisposition::Blocked)
    {
        OverallDisposition::Blocked
    } else {
        OverallDisposition::Pass
    };
    ProductFitnessDisposition {
        schema_version: "HarnessProductFitnessDisposition-v1".to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        producer_actor_id: "/root/plugin_lifecycle_product_engineer".to_owned(),
        reviewer_actor_id: "/root/product_journey_reviewer".to_owned(),
        owner_role: "product-journey-reviewer".to_owned(),
        reviewer_authority: "falsification_evidence_only".to_owned(),
        may_raise_claim_ceiling: false,
        dimensions,
        truth_layer_ceilings: TRUTH_LAYERS
            .into_iter()
            .map(|layer| (layer, ClaimCeiling::LiveSameSurfaceProven))
            .collect(),
        substitution_rejections: BTreeSet::from([
            "documentation_only".to_owned(),
            "fixture_pass".to_owned(),
            "install_success".to_owned(),
            "package_publication".to_owned(),
            "receipt_only".to_owned(),
            "reviewer_agreement".to_owned(),
            "smoke_test".to_owned(),
        ]),
        overall,
    }
}

#[test]
fn exact_five_dimensions_bind_current_same_surface_evidence() {
    let root = TempRoot::new();
    let disposition = disposition(root.path(), |_| DimensionDisposition::Pass);
    disposition.validate(root.path()).unwrap();
    assert_eq!(disposition.dimensions.len(), 5);
    assert_eq!(disposition.overall, OverallDisposition::Pass);
    assert!(!disposition.may_raise_claim_ceiling);
}

#[test]
fn blocked_and_failed_dimensions_lower_the_overall_disposition() {
    let root = TempRoot::new();
    disposition(root.path(), |dimension| {
        if dimension == FitnessDimension::RealUseEvidence {
            DimensionDisposition::Blocked
        } else {
            DimensionDisposition::Pass
        }
    })
    .validate(root.path())
    .unwrap();
    disposition(root.path(), |dimension| {
        if dimension == FitnessDimension::RecoveryBurden {
            DimensionDisposition::Fail
        } else {
            DimensionDisposition::Pass
        }
    })
    .validate(root.path())
    .unwrap();
}

#[test]
fn missing_duplicate_or_inconsistent_dimension_dispositions_fail_closed() {
    let root = TempRoot::new();
    let mut missing = disposition(root.path(), |_| DimensionDisposition::Pass);
    missing.dimensions.pop();
    assert_eq!(
        missing.validate(root.path()),
        Err(ProductFitnessError::MissingDimension)
    );
    let mut duplicate = disposition(root.path(), |_| DimensionDisposition::Pass);
    duplicate.dimensions[4].dimension = FitnessDimension::Accessibility;
    assert_eq!(
        duplicate.validate(root.path()),
        Err(ProductFitnessError::DuplicateDimension)
    );
    let mut inconsistent = disposition(root.path(), |_| DimensionDisposition::Pass);
    inconsistent.overall = OverallDisposition::Blocked;
    assert_eq!(
        inconsistent.validate(root.path()),
        Err(ProductFitnessError::InvalidDisposition)
    );
}

#[test]
fn evidence_is_candidate_bound_digest_bound_and_current_for_a_pass() {
    let root = TempRoot::new();
    let mut wrong_candidate = disposition(root.path(), |_| DimensionDisposition::Pass);
    wrong_candidate.dimensions[0].evidence.candidate_id =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
    assert_eq!(
        wrong_candidate.validate(root.path()),
        Err(ProductFitnessError::CandidateMismatch)
    );

    let mut stale_digest = disposition(root.path(), |_| DimensionDisposition::Pass);
    stale_digest.dimensions[0].evidence.sha256 =
        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
    assert_eq!(
        stale_digest.validate(root.path()),
        Err(ProductFitnessError::EvidenceDigestMismatch)
    );

    let mut prior_session = disposition(root.path(), |_| DimensionDisposition::Pass);
    prior_session.dimensions[0].evidence.current_session = false;
    assert_eq!(
        prior_session.validate(root.path()),
        Err(ProductFitnessError::EvidenceStale)
    );
}

#[test]
fn reviewer_is_disjoint_falsification_only_and_cannot_raise_claims() {
    let root = TempRoot::new();
    let mut same_actor = disposition(root.path(), |_| DimensionDisposition::Pass);
    same_actor.reviewer_actor_id = same_actor.producer_actor_id.clone();
    assert_eq!(
        same_actor.validate(root.path()),
        Err(ProductFitnessError::ActorNotDisjoint)
    );
    let mut authority = disposition(root.path(), |_| DimensionDisposition::Pass);
    authority.may_raise_claim_ceiling = true;
    assert_eq!(
        authority.validate(root.path()),
        Err(ProductFitnessError::ReviewerAuthorityInvalid)
    );
}

#[test]
fn every_host_truth_layer_is_separate_and_cannot_skip_an_unproven_predecessor() {
    let root = TempRoot::new();
    let mut disposition = disposition(root.path(), |_| DimensionDisposition::Blocked);
    disposition.truth_layer_ceilings = source_candidate_ceilings();
    disposition.validate(root.path()).unwrap();
    assert_eq!(disposition.truth_layer_ceilings.len(), 10);
    assert_eq!(
        disposition.truth_layer_ceilings[&TruthLayer::PluginsUi],
        ClaimCeiling::Withheld
    );

    disposition
        .truth_layer_ceilings
        .insert(TruthLayer::Runtime, ClaimCeiling::LiveSameSurfaceProven);
    assert_eq!(
        disposition.validate(root.path()),
        Err(ProductFitnessError::TruthLayerEscalation)
    );
}

#[cfg(unix)]
#[test]
fn symlink_evidence_is_rejected_as_a_special_file() {
    use std::os::unix::fs::symlink;
    let root = TempRoot::new();
    let mut disposition = disposition(root.path(), |_| DimensionDisposition::Pass);
    symlink("evidence.json", root.path().join("alias.json")).unwrap();
    for dimension in &mut disposition.dimensions {
        dimension.evidence.path = "alias.json".to_owned();
    }
    assert_eq!(
        disposition.validate(root.path()),
        Err(ProductFitnessError::EvidenceSpecialFile)
    );
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FitnessCases {
    schema_version: String,
    required_dimensions: Vec<FitnessDimension>,
    truth_layers: Vec<TruthLayer>,
    unsupported_ceiling: ClaimCeiling,
    substitution_rejections: Vec<String>,
}

#[test]
fn product_fitness_fixture_names_exact_dimensions_layers_and_substitutions() {
    let fixture: FitnessCases = serde_json::from_str(&super::read(
        "fixtures/plugin-product/product-fitness-cases.json",
    ))
    .unwrap();
    assert_eq!(fixture.schema_version, "HarnessProductFitnessCases-v1");
    assert_eq!(fixture.required_dimensions.len(), 5);
    assert_eq!(fixture.truth_layers, TRUTH_LAYERS);
    assert_eq!(fixture.unsupported_ceiling, ClaimCeiling::Withheld);
    assert!(
        fixture
            .substitution_rejections
            .contains(&"receipt_only".to_owned())
    );
}

#[test]
fn typed_disposition_rejects_unknown_fields() {
    let root = TempRoot::new();
    let value =
        serde_json::to_value(disposition(root.path(), |_| DimensionDisposition::Pass)).unwrap();
    let mut value = value.as_object().unwrap().clone();
    value.insert("unknown".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<ProductFitnessDisposition>(value.into()).is_err());

    let mut wrong_schema = disposition(root.path(), |_| DimensionDisposition::Pass);
    wrong_schema.schema_version = "future".to_owned();
    assert_eq!(
        wrong_schema.validate(root.path()),
        Err(ProductFitnessError::InvalidDisposition)
    );
}
