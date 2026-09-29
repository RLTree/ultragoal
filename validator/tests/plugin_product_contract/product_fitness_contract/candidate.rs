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
            "legacy_dogfood_receipt".to_owned(),
            "package_publication".to_owned(),
            "receipt_only".to_owned(),
            "reviewer_agreement".to_owned(),
            "smoke_test".to_owned(),
        ]),
        overall,
        operator_kind: None,
        evidence_class: None,
        surface_identities: None,
        public_entry_observation: None,
        real_work_observation: None,
        manual_journey_row: None,
        claimed_surface: None,
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
