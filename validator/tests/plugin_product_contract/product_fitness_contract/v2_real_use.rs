fn observed_surface(root: &Path, name: &str, identity: &str) -> SurfaceIdentity {
    let path = format!("{name}.json");
    let bytes = format!("{identity}\n");
    fs::write(root.join(&path), bytes.as_bytes()).unwrap();
    SurfaceIdentity {
        status: SurfaceStatus::Observed,
        identity: Some(identity.to_owned()),
        evidence: Some(EvidenceBinding {
            path,
            sha256: digest(bytes.as_bytes()),
            candidate_id: CANDIDATE.to_owned(),
            same_surface: true,
            current_session: true,
        }),
    }
}

fn withheld_surface() -> SurfaceIdentity {
    SurfaceIdentity {
        status: SurfaceStatus::Withheld,
        identity: None,
        evidence: None,
    }
}

fn v2_disposition(root: &Path) -> ProductFitnessDisposition {
    let mut disposition = disposition(root, |_| DimensionDisposition::Pass);
    disposition.schema_version = "HarnessProductFitnessDisposition-v2".to_owned();
    disposition.truth_layer_ceilings = source_candidate_ceilings();
    disposition.operator_kind = Some(OperatorKind::Human);
    disposition.evidence_class = Some(EvidenceClass::RepeatedHumanUse);
    disposition.surface_identities = Some(SurfaceIdentities {
        source: observed_surface(root, "source", "source-identity"),
        package: observed_surface(root, "package", "package-identity"),
        marketplace: withheld_surface(),
        install: observed_surface(root, "install", "install-identity"),
        cache: observed_surface(root, "cache", "cache-identity"),
        app_registry: observed_surface(root, "app-registry", "app-registry-identity"),
        discovery: observed_surface(root, "discovery", "discovery-identity"),
        runtime: observed_surface(root, "runtime", "runtime-identity"),
        journey: observed_surface(root, "journey", "journey-identity"),
    });
    disposition.public_entry_observation = Some(PublicEntryObservation {
        surface_id: "PS-ENTRY".to_owned(),
        route: "harness-ultragoal".to_owned(),
        bypass_attempted: true,
        bypass_rejected: true,
    });
    disposition.real_work_observation = Some(RealWorkObservation {
        repository_identity:
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        task_id: "task-1".to_owned(),
        task: "complete a real repository task".to_owned(),
        useful_outcome: "verified repository outcome".to_owned(),
    });
    disposition.manual_journey_row = Some(ManualJourneyRow {
        time_to_verified_value_ms: 10,
        human_interventions: 1,
        review_rounds: 1,
        failure: "representative failure".to_owned(),
        diagnosis: "diagnosed from the command result".to_owned(),
        recovery_outcome: "recovered with unrelated work preserved".to_owned(),
        repeat_use_outcome: "human repeated the useful task".to_owned(),
        retained_artifact_bytes: 12,
        retained_cache_bytes: 24,
        false_passes: 0,
        false_rejections: 0,
    });
    disposition
}

#[test]
fn v2_binds_real_use_and_separate_surface_identities() {
    let root = TempRoot::new();
    let disposition = v2_disposition(root.path());
    disposition.validate(root.path()).unwrap();
}

#[test]
fn v2_rejects_agent_use_relabelled_as_human_continuance() {
    let root = TempRoot::new();
    let mut disposition = v2_disposition(root.path());
    disposition.operator_kind = Some(OperatorKind::Agent);
    disposition.evidence_class = Some(EvidenceClass::AgentUse);
    assert_eq!(
        disposition.validate(root.path()),
        Err(ProductFitnessError::AgentUseRelabeledContinuance)
    );
}

#[test]
fn v2_rejects_bypass_wrong_surface_missing_identity_and_unsupported_ceiling() {
    let root = TempRoot::new();
    let mut bypass = v2_disposition(root.path());
    bypass
        .public_entry_observation
        .as_mut()
        .unwrap()
        .bypass_rejected = false;
    assert_eq!(
        bypass.validate(root.path()),
        Err(ProductFitnessError::BypassAttempt)
    );

    let mut wrong_surface = v2_disposition(root.path());
    wrong_surface
        .surface_identities
        .as_mut()
        .unwrap()
        .runtime
        .evidence
        .as_mut()
        .unwrap()
        .same_surface = false;
    assert_eq!(
        wrong_surface.validate(root.path()),
        Err(ProductFitnessError::WrongSurface)
    );

    let mut missing = v2_disposition(root.path());
    missing.surface_identities.as_mut().unwrap().journey.status = SurfaceStatus::Withheld;
    missing
        .surface_identities
        .as_mut()
        .unwrap()
        .journey
        .identity = None;
    missing
        .surface_identities
        .as_mut()
        .unwrap()
        .journey
        .evidence = None;
    missing
        .truth_layer_ceilings
        .insert(TruthLayer::Journey, ClaimCeiling::LiveSameSurfaceProven);
    assert_eq!(
        missing.validate(root.path()),
        Err(ProductFitnessError::MissingObservation)
    );

    let mut unsupported = v2_disposition(root.path());
    unsupported.evidence_class = Some(EvidenceClass::Intent);
    unsupported.dimensions.iter_mut().for_each(|item| {
        if item.dimension == FitnessDimension::Continuance {
            item.disposition = DimensionDisposition::Blocked;
        }
    });
    unsupported.overall = OverallDisposition::Blocked;
    unsupported
        .truth_layer_ceilings
        .insert(TruthLayer::Source, ClaimCeiling::LiveSameSurfaceProven);
    assert_eq!(
        unsupported.validate(root.path()),
        Err(ProductFitnessError::UnsupportedClaimCeiling)
    );
}

#[test]
fn typed_disposition_rejects_legacy_dogfood_schema() {
    let root = TempRoot::new();
    let mut legacy = v2_disposition(root.path());
    legacy.schema_version = "harness-ultragoal.dogfood-receipt.v1".to_owned();
    assert_eq!(
        legacy.validate(root.path()),
        Err(ProductFitnessError::InvalidDisposition)
    );
}
