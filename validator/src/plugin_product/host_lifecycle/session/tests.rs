use super::*;
use crate::host_fixture::{Fixture, Reader, installed, lifecycle, request};

struct IssuanceRaceReader {
    issuance: Arc<SessionIssuance>,
    transaction_calls: usize,
}

impl HostSurfaceReader for IssuanceRaceReader {
    fn with_transaction<T>(
        &mut self,
        _request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        self.transaction_calls += 1;
        self.issuance.reject();
        let mut transaction = NeverTouchedTransaction;
        operation(Ok(&mut transaction))
    }
}

struct NeverTouchedTransaction;

impl HostSurfaceTransaction for NeverTouchedTransaction {
    fn provenance_sha256(&self) -> &str {
        panic!("stale issuance reached transaction provenance")
    }

    fn host_scope_sha256(&self) -> &str {
        panic!("stale issuance reached transaction scope")
    }

    fn session_issuance_sha256(&self) -> &str {
        panic!("stale issuance reached transaction issuance")
    }

    fn start_generation(&self) -> u64 {
        panic!("stale issuance reached transaction generation")
    }

    fn current_generation(&self) -> Result<u64, ()> {
        panic!("stale issuance reached current generation")
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("stale issuance reached marketplace read")
    }

    fn read_cache(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("stale issuance reached cache read")
    }

    fn read_registry(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("stale issuance reached registry read")
    }

    fn read_plugins_ui(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("stale issuance reached Plugins UI read")
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        panic!("stale issuance reached runtime read")
    }
}

#[test]
fn issuance_mutation_after_preflight_fails_before_transaction_metadata_or_reads() {
    let fixture = Fixture::new("preflight-to-transaction-issuance-race");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 24);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, _) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();
    let mut reader = IssuanceRaceReader {
        issuance: Arc::clone(&session.issuance),
        transaction_calls: 0,
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
    assert_eq!(reader.transaction_calls, 1);
    assert_eq!(fixture.tree(), before);
}

#[test]
fn private_frames_reject_both_scope_directions_and_same_scope_siblings() {
    let fixture = Fixture::new("private-observation-frame-scope-seal");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 13);
    let repeat = || {
        lifecycle(
            &state,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        )
    };
    let host = fixture.host();
    let personal_scope = HostScopeAuthority::Personal {
        marketplace: "local-harness-plugins".into(),
    };
    let repository_scope = HostScopeAuthority::Repository {
        repository_root: fixture.project.to_string_lossy().into_owned(),
        marketplace: "local-harness-plugins".into(),
    };
    let mut personal_owner = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &bundle.plan,
        package: &bundle.snapshot,
        lifecycle: repeat(),
        host: host.clone(),
        marketplace_plan: crate::host_fixture::marketplace_plan(&bundle),
        host_scope: personal_scope.clone(),
    })
    .unwrap();
    let mut personal_sibling = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &bundle.plan,
        package: &bundle.snapshot,
        lifecycle: repeat(),
        host: host.clone(),
        marketplace_plan: crate::host_fixture::marketplace_plan(&bundle),
        host_scope: personal_scope,
    })
    .unwrap();
    let mut repository = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &bundle.plan,
        package: &bundle.snapshot,
        lifecycle: repeat(),
        host: host.clone(),
        marketplace_plan: crate::host_fixture::marketplace_plan(&bundle),
        host_scope: repository_scope,
    })
    .unwrap();
    for session in [&mut personal_owner, &mut personal_sibling, &mut repository] {
        session.apply_confined(&state).unwrap();
    }
    let template = Reader::complete(&bundle, &host, personal_owner.binding());
    let personal_frame = personal_owner
        .capture_observations_for_test(&mut template.clone())
        .unwrap();
    let sibling_frame = personal_sibling
        .capture_observations_for_test(&mut template.clone())
        .unwrap();
    let repository_frame = repository
        .capture_observations_for_test(&mut template.clone())
        .unwrap();
    assert_eq!(
        personal_frame.binding_sha256(),
        repository_frame.binding_sha256()
    );
    assert_eq!(
        personal_frame.host_scope_sha256(),
        sibling_frame.host_scope_sha256()
    );
    assert_ne!(
        personal_frame.session_issuance_sha256(),
        sibling_frame.session_issuance_sha256()
    );
    assert!(!format!("{personal_frame:?}").contains(&fixture.project.to_string_lossy()[..]));
    let before = fixture.tree();
    for error in [
        repository
            .verify_observations(personal_frame, state.clone())
            .unwrap_err(),
        personal_sibling
            .verify_observations(repository_frame, state.clone())
            .unwrap_err(),
        personal_owner
            .verify_observations(sibling_frame, state.clone())
            .unwrap_err(),
    ] {
        assert_eq!(error.id(), HostLifecycleErrorId::IdentityMismatch);
        assert!(
            !error
                .to_string()
                .contains(&fixture.project.to_string_lossy()[..])
        );
    }
    assert_eq!(fixture.tree(), before);
}
