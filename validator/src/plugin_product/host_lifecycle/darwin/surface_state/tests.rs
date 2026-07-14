use super::{
    DarwinHostDiagnosis, DarwinHostErrorId, DarwinHostSnapshot, DarwinHostSurface,
    DarwinSurfaceObservation, DarwinSurfaceStatus,
};

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn exact_keyed_surface_set_accepts_every_surface_once_in_any_order() {
    let mut rows = DarwinHostSurface::ALL
        .into_iter()
        .map(DarwinSurfaceObservation::absent)
        .collect::<Vec<_>>();
    rows.reverse();
    let snapshot = DarwinHostSnapshot::assemble(rows, None).expect("exact keyed set");
    assert_eq!(snapshot.diagnosis(), DarwinHostDiagnosis::Absent);
}

#[test]
fn duplicate_or_missing_surface_is_rejected() {
    let mut rows = DarwinHostSurface::ALL
        .into_iter()
        .map(DarwinSurfaceObservation::absent)
        .collect::<Vec<_>>();
    rows.pop();
    rows.push(DarwinSurfaceObservation::absent(
        DarwinHostSurface::Installed,
    ));
    assert_eq!(
        DarwinHostSnapshot::assemble(rows, None).unwrap_err().id(),
        DarwinHostErrorId::SurfaceConflict,
    );
}

#[test]
fn digest_type_rejects_uppercase_and_verified_state_requires_complete_binding() {
    let uppercase = DarwinSurfaceObservation::verified(
        DarwinHostSurface::Installed,
        format!("sha256:{}", "A".repeat(64)),
        digest('b'),
        "0.0.12".to_owned(),
    );
    assert_eq!(
        uppercase.unwrap_err().id(),
        DarwinHostErrorId::SurfaceConflict
    );

    let candidate_id = digest('b');
    let verified = DarwinSurfaceObservation::verified(
        DarwinHostSurface::Installed,
        digest('a'),
        candidate_id.clone(),
        "0.0.12".to_owned(),
    )
    .expect("complete verified state");
    assert_eq!(verified.status(), DarwinSurfaceStatus::Verified);
    assert_eq!(
        verified.observed_candidate_id(),
        Some(candidate_id.as_str())
    );
}

#[test]
fn conflict_state_cannot_masquerade_as_absent_or_verified() {
    for status in [DarwinSurfaceStatus::Absent, DarwinSurfaceStatus::Verified] {
        assert_eq!(
            DarwinSurfaceObservation::conflict(
                DarwinHostSurface::Runtime,
                status,
                digest('c'),
                None,
                None,
            )
            .unwrap_err()
            .id(),
            DarwinHostErrorId::SurfaceConflict,
        );
    }
}
