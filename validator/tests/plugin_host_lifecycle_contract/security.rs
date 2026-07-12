use crate::host_lifecycle::{
    HostLifecycleErrorId, HostObservationTransactionRequest, HostScopeAuthority, HostSurfaceReader,
    HostSurfaceTransaction, HostSurfaceTransactionError,
};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};
use crate::support::{Fixture, Reader, installed, lifecycle, request};

#[test]
fn sealed_plan_replay_and_sibling_sessions_authorize_exactly_one_transition() {
    let fixture = Fixture::new("session-replay");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut owner, host) = fixture.session(&bundle, plan.clone());
    let (mut sibling, _) = fixture.session(&bundle, plan);
    let before = fixture.tree();
    assert_eq!(
        owner.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(fixture.tree(), before);
    owner.apply_confined(&empty).unwrap();
    let after_owner = fixture.tree();
    let mut reader = Reader::complete(&bundle, &host, owner.binding());
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(
        sibling.apply_confined(&empty).unwrap_err().id(),
        HostLifecycleErrorId::LifecycleRejected
    );
    assert_eq!(
        owner.apply_confined(&empty).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(
        sibling.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    let request = owner.take_external_effect_request().unwrap();
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(
        owner.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectReplayed
    );
    let prepared = owner.consume_external_effect_request(request).unwrap();
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(prepared.into_plan().unwrap().commands().len(), 1);
    assert!(owner.capture_and_verify(&mut reader).is_ok());
    assert_eq!(fixture.tree(), after_owner);
}

#[test]
fn independently_issued_identical_sessions_have_unique_requests_and_refuse_cross_session_use() {
    let fixture = Fixture::new("independent-session-crossing");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let first_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let second_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    assert_eq!(first_plan.plan_id, second_plan.plan_id);
    assert_ne!(
        first_plan, second_plan,
        "sealed issuances unexpectedly equal"
    );
    let (mut first, _) = fixture.session(&bundle, first_plan);
    let (mut second, _) = fixture.session(&bundle, second_plan);

    first.apply_confined(&empty).unwrap();
    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    second.apply_confined(&empty).unwrap();

    let first_request = first.take_external_effect_request().unwrap();
    let second_request = second.take_external_effect_request().unwrap();
    assert_ne!(
        first_request.session_issuance_sha256(),
        second_request.session_issuance_sha256()
    );
    assert_ne!(
        first_request.request_sha256(),
        second_request.request_sha256()
    );
    let before_cross = fixture.tree();
    assert_eq!(
        first
            .consume_external_effect_request(second_request)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ExternalEffectSessionMismatch
    );
    assert_eq!(fixture.tree(), before_cross);
    let prepared = first
        .consume_external_effect_request(first_request)
        .unwrap();
    assert_eq!(prepared.into_plan().unwrap().commands().len(), 1);
    assert_eq!(
        second.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectReplayed
    );
    assert_eq!(fixture.tree(), before_cross);
}

#[cfg(unix)]
#[test]
fn link_hardlink_fifo_and_socket_install_objects_refuse_without_outside_write() {
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    for kind in ["symlink", "hardlink", "fifo", "socket"] {
        let fixture = Fixture::new(&format!("special-{kind}"));
        let bundle = fixture.bundle("0.0.12");
        let installed_dir = fixture.root.join("installed");
        std::fs::create_dir_all(&installed_dir).unwrap();
        let target = installed_dir.join("harness-ultragoal.hugpkg");
        let outside = fixture.root.join("outside-canary");
        std::fs::write(&outside, b"outside-canary\n").unwrap();
        let _listener = match kind {
            "symlink" => {
                symlink(&outside, &target).unwrap();
                None
            }
            "hardlink" => {
                std::fs::hard_link(&outside, &target).unwrap();
                None
            }
            "fifo" => {
                assert!(
                    std::process::Command::new("mkfifo")
                        .arg(&target)
                        .status()
                        .unwrap()
                        .success()
                );
                None
            }
            "socket" => Some(UnixListener::bind(&target).unwrap()),
            _ => unreachable!(),
        };
        let logical = installed(&bundle.authority, 3);
        let repeat = lifecycle(
            &logical,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                logical.installed.as_ref(),
                false,
                false,
            ),
        );
        let before = std::fs::read(&outside).unwrap();
        let result = std::panic::catch_unwind(|| fixture.session(&bundle, repeat));
        assert!(result.is_err(), "{kind} unexpectedly bound");
        assert_eq!(std::fs::read(&outside).unwrap(), before);
    }
}

#[test]
fn oversized_and_attacker_observations_fail_without_echo_or_mutation() {
    let fixture = Fixture::new("attacker-observation");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 4);
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
    let canary = "SECRET_ATTACKER_CANARY";
    let mut reader = Reader {
        provenance_sha256: session.binding().binding_sha256().to_owned(),
        marketplace: Some(format!("{canary}{}", "x".repeat(4 * 1024 * 1024)).into_bytes()),
        ..Reader::default()
    };
    let before = fixture.tree();
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::ObservationConflict);
    assert!(!error.to_string().contains(canary));
    assert_eq!(fixture.tree(), before);
}

#[test]
fn external_effect_requests_are_candidate_bound_and_never_authorizations() {
    let fixture = Fixture::new("effect-request-binding");
    let first = fixture.bundle("0.0.11");
    let second = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let first_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(first.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut first_session, _) = fixture.session(&first, first_plan);
    first_session.apply_confined(&empty).unwrap();
    let first_request = first_session.take_external_effect_request().unwrap();

    let other_fixture = Fixture::new("effect-request-binding-other");
    let other_second = other_fixture.bundle("0.0.12");
    let second_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(other_second.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut second_session, _) = other_fixture.session(&other_second, second_plan);
    second_session.apply_confined(&empty).unwrap();
    let second_request = second_session.take_external_effect_request().unwrap();
    assert_ne!(
        first_request.request_sha256(),
        second_request.request_sha256()
    );
    assert_ne!(
        first.snapshot.package_sha256(),
        second.snapshot.package_sha256()
    );
    assert!(!first_request.plan_sha256().is_empty());
    let serialized = serde_json::to_value(&first_request).unwrap();
    assert_eq!(
        serialized["session_issuance_sha256"],
        first_request.session_issuance_sha256()
    );
    assert!(serialized.get("commands").is_none());
    assert!(serialized.get("plan").is_none());
    let first_prepared = first_session
        .consume_external_effect_request(first_request)
        .unwrap();
    let first_host_plan = first_prepared.into_plan().unwrap();
    assert_eq!(first_host_plan.commands()[0].program(), "codex");
    let second_prepared = second_session
        .consume_external_effect_request(second_request)
        .unwrap();
    assert_eq!(second_prepared.into_plan().unwrap().commands().len(), 1);
}

#[test]
fn observation_capture_mutation_is_zero_write_and_does_not_consume_session() {
    let fixture = Fixture::new("capture-mutation");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 5);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();
    let mut raced = Reader::complete(&bundle, &host, session.binding());
    raced.mutate_marketplace_after_first = true;
    assert_eq!(
        session.capture_and_verify(&mut raced).unwrap_err().id(),
        HostLifecycleErrorId::ObservationChanged
    );
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
}

trait TestSurfaceSource {
    fn provenance_sha256(&self) -> &str;
    fn generation(&self) -> u64 {
        0
    }
    fn transaction_supported(&self) -> bool {
        true
    }
    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()>;
}

struct TestSurfaceTransaction<'a, S> {
    source: &'a mut S,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl<S: TestSurfaceSource> HostSurfaceTransaction for TestSurfaceTransaction<'_, S> {
    fn provenance_sha256(&self) -> &str {
        self.source.provenance_sha256()
    }

    fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.source.generation())
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_marketplace(maximum)
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_cache(maximum)
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_registry(maximum)
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.source.read_plugins_ui(maximum)
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.source.read_runtime()
    }
}

fn with_test_transaction<S: TestSurfaceSource, T>(
    source: &mut S,
    request: &HostObservationTransactionRequest,
    operation: impl FnOnce(Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>) -> T,
) -> T {
    if !source.transaction_supported() {
        return operation(Err(HostSurfaceTransactionError::Unsupported));
    }
    let start_generation = source.generation();
    let mut transaction = TestSurfaceTransaction {
        source,
        host_scope_sha256: request.host_scope_sha256().to_owned(),
        session_issuance_sha256: request.session_issuance_sha256().to_owned(),
        start_generation,
    };
    operation(Ok(&mut transaction))
}

#[derive(Debug, Default)]
struct AdapterInvocationCounts {
    transactions: std::cell::Cell<usize>,
    provenance: std::cell::Cell<usize>,
    scope: std::cell::Cell<usize>,
    issuance: std::cell::Cell<usize>,
    start_generation: std::cell::Cell<usize>,
    current_generation: std::cell::Cell<usize>,
    marketplace_reads: std::cell::Cell<usize>,
    cache_reads: std::cell::Cell<usize>,
    registry_reads: std::cell::Cell<usize>,
    ui_reads: std::cell::Cell<usize>,
    runtime_reads: std::cell::Cell<usize>,
}

impl AdapterInvocationCounts {
    fn increment(cell: &std::cell::Cell<usize>) {
        cell.set(cell.get() + 1);
    }

    fn is_zero(&self) -> bool {
        [
            &self.transactions,
            &self.provenance,
            &self.scope,
            &self.issuance,
            &self.start_generation,
            &self.current_generation,
            &self.marketplace_reads,
            &self.cache_reads,
            &self.registry_reads,
            &self.ui_reads,
            &self.runtime_reads,
        ]
        .into_iter()
        .all(|value| value.get() == 0)
    }

    fn read_count(&self) -> usize {
        self.marketplace_reads.get()
            + self.cache_reads.get()
            + self.registry_reads.get()
            + self.ui_reads.get()
            + self.runtime_reads.get()
    }
}

#[derive(Default)]
struct CountingUnsupportedReader {
    counts: AdapterInvocationCounts,
}

impl HostSurfaceReader for CountingUnsupportedReader {
    fn with_transaction<T>(
        &mut self,
        _request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        AdapterInvocationCounts::increment(&self.counts.transactions);
        operation(Err(HostSurfaceTransactionError::Unsupported))
    }
}

struct CountingMutationReader {
    inner: Reader,
    counts: AdapterInvocationCounts,
    before_transaction: Option<Box<dyn FnOnce()>>,
}

impl HostSurfaceReader for CountingMutationReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        AdapterInvocationCounts::increment(&self.counts.transactions);
        if let Some(mutation) = self.before_transaction.take() {
            mutation();
        }
        let start_generation = self.inner.generation;
        let mut transaction = CountingSurfaceTransaction {
            reader: &mut self.inner,
            counts: &self.counts,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

struct CountingSurfaceTransaction<'a> {
    reader: &'a mut Reader,
    counts: &'a AdapterInvocationCounts,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl HostSurfaceTransaction for CountingSurfaceTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.provenance);
        &self.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.scope);
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.issuance);
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        AdapterInvocationCounts::increment(&self.counts.start_generation);
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        AdapterInvocationCounts::increment(&self.counts.current_generation);
        Ok(self.reader.generation)
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.marketplace_reads);
        self.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.cache_reads);
        self.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.registry_reads);
        self.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.ui_reads);
        self.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        AdapterInvocationCounts::increment(&self.counts.runtime_reads);
        self.reader.read_runtime_raw()
    }
}

#[test]
fn invalid_local_preflight_states_make_exactly_zero_adapter_calls() {
    {
        let fixture = Fixture::new("preflight-not-applied");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 18);
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
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ExternalEffectNotEligible);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-closed");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 19);
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
        let (mut session, host) = fixture.session(&bundle, repeat);
        session.apply_confined(&state).unwrap();
        let mut current = Reader::complete(&bundle, &host, session.binding());
        session.capture_and_verify(&mut current).unwrap();
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-effect-not-ready");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let (mut session, _) = fixture.session(&bundle, fresh);
        session.apply_confined(&empty).unwrap();
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ExternalEffectNotEligible);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-unsupported-capability");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let unavailable = crate::distribution::HostCapabilityDeclaration::unavailable_codex_app(
            &fixture.root,
            &fixture.project,
            "UNSUPPORTED_PREFLIGHT_CANARY",
        )
        .unwrap();
        let mut session = fixture.session_with_scope(
            &bundle,
            fresh,
            unavailable,
            HostScopeAuthority::Personal {
                marketplace: "local-harness-plugins".into(),
            },
        );
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::HostCapabilityRejected);
        assert!(!error.to_string().contains("CANARY"));
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn stale_scope_rejected_session_wrong_state_and_effect_replay_never_reach_adapter() {
    {
        let fixture = Fixture::new("preflight-stale-scope-rejected");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 20);
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
        let host = fixture.host();
        let mut session = fixture.session_with_scope(
            &bundle,
            repeat,
            host,
            HostScopeAuthority::Repository {
                repository_root: fixture.project.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
        );
        session.apply_confined(&state).unwrap();
        let original = fixture.root.join("preflight-original-project");
        std::fs::rename(&fixture.project, &original).unwrap();
        std::fs::create_dir(&fixture.project).unwrap();
        let before = fixture.tree();
        let mut stale = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut stale).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
        assert!(stale.counts.is_zero());
        assert_eq!(fixture.tree(), before);

        std::fs::remove_dir(&fixture.project).unwrap();
        std::fs::rename(&original, &fixture.project).unwrap();
        let restored = fixture.tree();
        let mut rejected = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut rejected).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(rejected.counts.is_zero());
        assert_eq!(fixture.tree(), restored);
    }

    {
        let fixture = Fixture::new("preflight-wrong-confined-state");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 21);
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
        fixture.replace(
            "installed/harness-ultragoal.hugpkg",
            Some(&bundle.authority.package_sha256),
            None,
        );
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::IdentityMismatch);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-effect-replay");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let (mut session, host) = fixture.session(&bundle, fresh);
        session.apply_confined(&empty).unwrap();
        let effect_request = session.take_external_effect_request().unwrap();
        session
            .consume_external_effect_request(effect_request)
            .unwrap()
            .into_plan()
            .unwrap();
        let mut current = Reader::complete(&bundle, &host, session.binding());
        session.capture_and_verify(&mut current).unwrap();
        let before = fixture.tree();
        let mut replay = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut replay).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(replay.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn mutation_after_preflight_is_rechecked_inside_transaction_before_surface_reads() {
    let fixture = Fixture::new("preflight-to-transaction-confined-state-race");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 22);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let installed_path = fixture.root.join("installed/harness-ultragoal.hugpkg");
    let mut reader = CountingMutationReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        counts: AdapterInvocationCounts::default(),
        before_transaction: Some(Box::new(move || {
            std::fs::remove_file(installed_path).unwrap();
        })),
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::IdentityMismatch);
    assert_eq!(reader.counts.transactions.get(), 1);
    assert!(reader.counts.start_generation.get() > 0);
    assert!(reader.counts.current_generation.get() > 0);
    assert_eq!(reader.counts.read_count(), 0);
}

#[test]
fn scope_mutation_after_preflight_is_rejected_before_transaction_metadata_or_reads() {
    let fixture = Fixture::new("preflight-to-transaction-scope-race");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 23);
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
    let host = fixture.host();
    let mut session = fixture.session_with_scope(
        &bundle,
        repeat,
        host.clone(),
        HostScopeAuthority::Repository {
            repository_root: fixture.project.to_string_lossy().into_owned(),
            marketplace: "local-harness-plugins".into(),
        },
    );
    session.apply_confined(&state).unwrap();
    let project = fixture.project.clone();
    let moved = fixture.root.join("transaction-race-original-project");
    let moved_for_callback = moved.clone();
    let mut reader = CountingMutationReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        counts: AdapterInvocationCounts::default(),
        before_transaction: Some(Box::new(move || {
            std::fs::rename(&project, &moved_for_callback).unwrap();
            std::fs::create_dir(&project).unwrap();
        })),
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
    assert_eq!(reader.counts.transactions.get(), 1);
    assert_eq!(reader.counts.provenance.get(), 0);
    assert_eq!(reader.counts.scope.get(), 0);
    assert_eq!(reader.counts.issuance.get(), 0);
    assert_eq!(reader.counts.start_generation.get(), 0);
    assert_eq!(reader.counts.current_generation.get(), 0);
    assert_eq!(reader.counts.read_count(), 0);

    std::fs::remove_dir(&fixture.project).unwrap();
    std::fs::rename(moved, &fixture.project).unwrap();
}

struct ScopeSwapReader {
    inner: Reader,
    project: std::path::PathBuf,
    moved: std::path::PathBuf,
    swapped: bool,
    swap_on_marketplace_read: usize,
    marketplace_reads: usize,
}

impl ScopeSwapReader {
    fn swap_once(&mut self) {
        if self.swapped {
            return;
        }
        std::fs::rename(&self.project, &self.moved).unwrap();
        std::fs::create_dir(&self.project).unwrap();
        self.swapped = true;
    }
}

impl HostSurfaceReader for ScopeSwapReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for ScopeSwapReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == self.swap_on_marketplace_read {
            self.swap_once();
        }
        let _ = maximum;
        self.inner.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

#[test]
fn repository_scope_mutation_during_capture_and_between_capture_verify_fails_closed() {
    for boundary in ["capture", "verify"] {
        let fixture = Fixture::new(&format!("observation-scope-race-{boundary}"));
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 7);
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
        let host = fixture.host();
        let mut session = fixture.session_with_scope(
            &bundle,
            repeat,
            host.clone(),
            HostScopeAuthority::Repository {
                repository_root: fixture.project.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
        );
        session.apply_confined(&state).unwrap();
        let original_tree = fixture.tree();
        let moved = fixture.root.join(format!("project-original-{boundary}"));
        let mut reader = ScopeSwapReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            project: fixture.project.clone(),
            moved: moved.clone(),
            swapped: false,
            swap_on_marketplace_read: if boundary == "capture" { 1 } else { 3 },
            marketplace_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        let mutated = fixture.tree();
        assert_eq!(
            fixture.tree(),
            mutated,
            "{boundary} race wrote after failure"
        );
        assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
        assert!(!error.to_string().contains(&moved.to_string_lossy()[..]));

        std::fs::remove_dir(&fixture.project).unwrap();
        std::fs::rename(&moved, &fixture.project).unwrap();
        assert_eq!(
            fixture.tree(),
            original_tree,
            "scope race changed retained state"
        );
        let mut current = Reader::complete(&bundle, &host, session.binding());
        assert_eq!(
            session.capture_and_verify(&mut current).unwrap_err().id(),
            HostLifecycleErrorId::SessionStateRejected,
            "scope rejection revived at {boundary}"
        );
        assert_eq!(fixture.tree(), original_tree);
    }
}

#[test]
fn marketplace_and_cache_substitution_remain_zero_write_and_non_echoing() {
    let fixture = Fixture::new("observation-marketplace-cache-substitution");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 8);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    let marketplace_canary = "attacker-marketplace-canary";
    let mut marketplace = Reader::complete(&bundle, &host, session.binding());
    marketplace.marketplace = Some(
        crate::support::marketplace_plan_named(&bundle, marketplace_canary)
            .replacement()
            .to_vec(),
    );
    let marketplace_error = session.capture_and_verify(&mut marketplace).unwrap_err();
    assert_eq!(
        marketplace_error.id(),
        HostLifecycleErrorId::ObservationConflict
    );
    assert!(!marketplace_error.to_string().contains(marketplace_canary));
    assert_eq!(fixture.tree(), before);

    let cache_canary = "attacker-cache-canary";
    let mut cache = Reader::complete(&bundle, &host, session.binding());
    let mut cache_document: serde_json::Value =
        serde_json::from_slice(cache.cache.as_ref().unwrap()).unwrap();
    cache_document["entries"][0]["marketplace"] = serde_json::json!(cache_canary);
    cache.cache = Some(serde_json::to_vec(&cache_document).unwrap());
    let cache_error = session.capture_and_verify(&mut cache).unwrap_err();
    assert_eq!(cache_error.id(), HostLifecycleErrorId::ObservationConflict);
    assert!(!cache_error.to_string().contains(cache_canary));
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before);
}

struct PostCaptureReader {
    before: Reader,
    after: Reader,
    surface: &'static str,
    marketplace_reads: usize,
    cache_reads: usize,
    registry_reads: usize,
    ui_reads: usize,
    runtime_reads: usize,
}

impl PostCaptureReader {
    fn use_after(surface: &str, expected: &str, reads: usize) -> bool {
        surface == expected && reads > 2
    }
}

impl HostSurfaceReader for PostCaptureReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for PostCaptureReader {
    fn provenance_sha256(&self) -> &str {
        if Self::use_after(self.surface, "provenance", self.marketplace_reads) {
            &self.after.provenance_sha256
        } else {
            &self.before.provenance_sha256
        }
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if Self::use_after(self.surface, "marketplace", self.marketplace_reads) {
            let _ = maximum;
            self.after.read_marketplace_raw()
        } else {
            let _ = maximum;
            self.before.read_marketplace_raw()
        }
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.cache_reads += 1;
        if Self::use_after(self.surface, "cache", self.cache_reads) {
            let _ = maximum;
            self.after.read_cache_raw()
        } else {
            let _ = maximum;
            self.before.read_cache_raw()
        }
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.registry_reads += 1;
        if Self::use_after(self.surface, self.surface, self.registry_reads)
            && matches!(self.surface, "app_registry" | "discovery")
        {
            let _ = maximum;
            self.after.read_registry_raw()
        } else {
            let _ = maximum;
            self.before.read_registry_raw()
        }
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.ui_reads += 1;
        if Self::use_after(self.surface, "plugins_ui", self.ui_reads) {
            let _ = maximum;
            self.after.read_plugins_ui_raw()
        } else {
            let _ = maximum;
            self.before.read_plugins_ui_raw()
        }
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.runtime_reads += 1;
        if Self::use_after(self.surface, "runtime", self.runtime_reads) {
            self.after.read_runtime_raw()
        } else {
            self.before.read_runtime_raw()
        }
    }
}

#[test]
fn every_host_surface_and_reader_provenance_is_reobserved_after_capture() {
    let fixture = Fixture::new("post-capture-all-surfaces");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 9);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before_tree = fixture.tree();

    for surface in [
        "marketplace",
        "cache",
        "app_registry",
        "plugins_ui",
        "discovery",
        "runtime",
        "provenance",
    ] {
        let before = Reader::complete(&bundle, &host, session.binding());
        let mut after = before.clone();
        let canary = format!("POST_CAPTURE_{surface}_CANARY");
        match surface {
            "marketplace" => after.marketplace = Some(canary.as_bytes().to_vec()),
            "cache" => after.cache = Some(canary.as_bytes().to_vec()),
            "app_registry" | "discovery" => after.registry = Some(canary.as_bytes().to_vec()),
            "plugins_ui" => after.ui = Some(canary.as_bytes().to_vec()),
            "runtime" => after.runtime = None,
            "provenance" => {
                after.provenance_sha256 =
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()
            }
            _ => unreachable!(),
        }
        let mut reader = PostCaptureReader {
            before,
            after,
            surface,
            marketplace_reads: 0,
            cache_reads: 0,
            registry_reads: 0,
            ui_reads: 0,
            runtime_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert!(matches!(
            error.id(),
            HostLifecycleErrorId::ObservationChanged
                | HostLifecycleErrorId::ObservationConflict
                | HostLifecycleErrorId::ObservationUnavailable
                | HostLifecycleErrorId::UnsupportedSubstitution
        ));
        assert!(!error.to_string().contains(&canary));
        assert_eq!(fixture.tree(), before_tree, "{surface} drift wrote");
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

struct TransientMarketplaceReader {
    stable: Reader,
    mutation: Option<Vec<u8>>,
    marketplace_reads: usize,
}

impl HostSurfaceReader for TransientMarketplaceReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for TransientMarketplaceReader {
    fn provenance_sha256(&self) -> &str {
        &self.stable.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == 3 {
            return Ok(self.mutation.clone());
        }
        let _ = maximum;
        self.stable.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.stable.read_runtime_raw()
    }
}

#[test]
fn post_capture_disappear_reappear_and_mutate_restore_are_refused_without_write() {
    let fixture = Fixture::new("post-capture-transient-drift");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 10);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before_tree = fixture.tree();
    for mutation in [None, Some(b"MUTATE_RESTORE_CANARY".to_vec())] {
        let mut reader = TransientMarketplaceReader {
            stable: Reader::complete(&bundle, &host, session.binding()),
            mutation,
            marketplace_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ObservationChanged);
        assert!(!error.to_string().contains("MUTATE_RESTORE_CANARY"));
        assert_eq!(fixture.tree(), before_tree);
    }
    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

struct InstalledDriftReader {
    inner: Reader,
    installed_path: std::path::PathBuf,
    replacement: Vec<u8>,
    marketplace_reads: usize,
}

impl HostSurfaceReader for InstalledDriftReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for InstalledDriftReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == 3 {
            std::fs::write(&self.installed_path, &self.replacement).unwrap();
        }
        let _ = maximum;
        self.inner.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

#[test]
fn installed_surface_is_reobserved_after_capture_before_decision() {
    let fixture = Fixture::new("post-capture-installed-drift");
    let bundle = fixture.bundle("0.0.12");
    let substitute = fixture.bundle("0.0.13");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 11);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let installed_path = fixture.root.join("installed/harness-ultragoal.hugpkg");
    let original = std::fs::read(&installed_path).unwrap();
    let before_tree = fixture.tree();
    let mut reader = InstalledDriftReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        installed_path: installed_path.clone(),
        replacement: substitute.snapshot.archive().to_vec(),
        marketplace_reads: 0,
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert!(matches!(
        error.id(),
        HostLifecycleErrorId::InvalidBinding
            | HostLifecycleErrorId::IdentityMismatch
            | HostLifecycleErrorId::ObservationChanged
    ));
    assert!(
        !error
            .to_string()
            .contains(&installed_path.to_string_lossy()[..])
    );
    std::fs::write(&installed_path, original).unwrap();
    assert_eq!(fixture.tree(), before_tree);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

#[derive(Clone, Copy)]
enum GenerationMutation {
    Increment,
    Rollback,
    MutateRestore,
}

struct GenerationMutationReader {
    inner: Reader,
    generation: u64,
    runtime_reads: usize,
    trigger_runtime_read: usize,
    mutation: GenerationMutation,
}

impl HostSurfaceReader for GenerationMutationReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for GenerationMutationReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.runtime_reads += 1;
        let observed = self.inner.read_runtime_raw();
        if self.runtime_reads == self.trigger_runtime_read {
            match self.mutation {
                GenerationMutation::Increment => {
                    self.inner.marketplace = Some(b"AFTER_LAST_LAYER_CANARY".to_vec());
                    self.generation += 1;
                }
                GenerationMutation::Rollback => self.generation -= 1,
                GenerationMutation::MutateRestore => {
                    let original = self.inner.marketplace.take();
                    self.inner.marketplace = Some(b"MUTATE_RESTORE_GENERATION_CANARY".to_vec());
                    self.inner.marketplace = original;
                    self.generation += 1;
                }
            }
        }
        observed
    }
}

#[test]
fn final_generation_revalidation_catches_after_last_layer_and_restore_attacks_without_close() {
    let fixture = Fixture::new("transaction-generation-close-races");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 14);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    for (label, trigger_runtime_read, generation, mutation) in [
        (
            "after-first-frame-last-layer",
            2,
            7,
            GenerationMutation::Increment,
        ),
        ("after-fourth-runtime", 4, 7, GenerationMutation::Increment),
        ("generation-rollback", 4, 7, GenerationMutation::Rollback),
        (
            "generation-mutate-restore",
            4,
            7,
            GenerationMutation::MutateRestore,
        ),
    ] {
        let mut reader = GenerationMutationReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            generation,
            runtime_reads: 0,
            trigger_runtime_read,
            mutation,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(
            error.id(),
            HostLifecycleErrorId::ObservationChanged,
            "{label}"
        );
        assert!(!error.to_string().contains("CANARY"), "{label}");
        assert_eq!(fixture.tree(), before, "{label} wrote");
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(fixture.tree(), before);
}

enum TransactionMismatch {
    ForgedScope,
    SiblingIssuance(String),
    StaleGeneration,
}

struct MismatchedTransactionReader {
    inner: Reader,
    mismatch: TransactionMismatch,
}

impl TestSurfaceSource for MismatchedTransactionReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn generation(&self) -> u64 {
        self.inner.generation
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

impl HostSurfaceReader for MismatchedTransactionReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        let mut host_scope_sha256 = request.host_scope_sha256().to_owned();
        let mut session_issuance_sha256 = request.session_issuance_sha256().to_owned();
        let mut start_generation = self.inner.generation;
        match &self.mismatch {
            TransactionMismatch::ForgedScope => {
                host_scope_sha256 =
                    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                        .into();
            }
            TransactionMismatch::SiblingIssuance(sibling) => {
                session_issuance_sha256 = sibling.clone();
            }
            TransactionMismatch::StaleGeneration => start_generation += 1,
        }
        let mut transaction = TestSurfaceTransaction {
            source: self,
            host_scope_sha256,
            session_issuance_sha256,
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

#[test]
fn forged_stale_and_sibling_transactions_cannot_bind_or_close_the_session() {
    let fixture = Fixture::new("transaction-binding-attacks");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 15);
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
    let (mut session, host) = fixture.session(&bundle, repeat());
    let (sibling, _) = fixture.session(&bundle, repeat());
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    for mismatch in [
        TransactionMismatch::ForgedScope,
        TransactionMismatch::SiblingIssuance(sibling.session_issuance_sha256().to_owned()),
        TransactionMismatch::StaleGeneration,
    ] {
        let mut reader = MismatchedTransactionReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            mismatch,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ObservationChanged);
        assert!(!error.to_string().contains("dddd"));
        assert_eq!(fixture.tree(), before);
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(fixture.tree(), before);
}

struct TransactionFailureReader {
    error: HostSurfaceTransactionError,
}

impl HostSurfaceReader for TransactionFailureReader {
    fn with_transaction<T>(
        &mut self,
        _request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        operation(Err(self.error))
    }
}

struct PanickingTransactionReader {
    inner: Reader,
}

impl HostSurfaceReader for PanickingTransactionReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for PanickingTransactionReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        panic!("transaction-reader-panic-canary")
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

#[test]
fn unsupported_failed_and_panicking_transactions_are_bounded_and_do_not_close() {
    let fixture = Fixture::new("transaction-failure-cleanup");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 16);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    let mut unsupported = TransactionFailureReader {
        error: HostSurfaceTransactionError::Unsupported,
    };
    let error = session.capture_and_verify(&mut unsupported).unwrap_err();
    assert_eq!(
        error.id(),
        HostLifecycleErrorId::ObservationTransactionUnsupported
    );
    assert!(!error.to_string().contains("canary"));

    let mut failed = TransactionFailureReader {
        error: HostSurfaceTransactionError::Failed,
    };
    let error = session.capture_and_verify(&mut failed).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::ObservationUnavailable);

    let mut panicking = PanickingTransactionReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
    };
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = session.capture_and_verify(&mut panicking);
    }));
    assert!(panic.is_err());
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(
        session.capture_and_verify(&mut current).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(fixture.tree(), before);
}

struct LockedSurfaceState {
    reader: Reader,
    generation: u64,
}

struct LockedSurfaceReader {
    state: std::sync::Arc<std::sync::Mutex<LockedSurfaceState>>,
    writer_ready: std::sync::Arc<std::sync::Barrier>,
}

struct LockedSurfaceTransaction<'a> {
    state: std::sync::MutexGuard<'a, LockedSurfaceState>,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl HostSurfaceReader for LockedSurfaceReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        let state = self.state.lock().unwrap();
        self.writer_ready.wait();
        let start_generation = state.generation;
        let mut transaction = LockedSurfaceTransaction {
            state,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

impl HostSurfaceTransaction for LockedSurfaceTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        &self.state.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.state.generation)
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.state.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.state.reader.read_runtime_raw()
    }
}

#[test]
fn reader_owned_transaction_serializes_concurrent_writer_through_session_close() {
    let fixture = Fixture::new("transaction-concurrent-writer");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 17);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(LockedSurfaceState {
        reader: Reader::complete(&bundle, &host, session.binding()),
        generation: 0,
    }));
    let writer_ready = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writer_state = std::sync::Arc::clone(&shared);
    let writer_barrier = std::sync::Arc::clone(&writer_ready);
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut state = writer_state.lock().unwrap();
        state.reader.marketplace = Some(b"POST_CLOSE_WRITER_CANARY".to_vec());
        state.generation += 1;
    });
    let mut reader = LockedSurfaceReader {
        state: std::sync::Arc::clone(&shared),
        writer_ready,
    };

    assert!(session.capture_and_verify(&mut reader).is_ok());
    writer.join().unwrap();
    assert_eq!(shared.lock().unwrap().generation, 1);
    let mut replay = Reader::complete(&bundle, &host, session.binding());
    assert_eq!(
        session.capture_and_verify(&mut replay).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(fixture.tree(), before);
}

#[test]
fn concurrent_capture_and_verify_has_one_winner_and_one_closed_loser() {
    let fixture = Fixture::new("concurrent-observation-transaction");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 12);
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
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let reader = Reader::complete(&bundle, &host, session.binding());
    let before_tree = fixture.tree();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(session));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut handles = Vec::new();
    for mut reader in [reader.clone(), reader] {
        let shared = std::sync::Arc::clone(&shared);
        let barrier = std::sync::Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            shared
                .lock()
                .unwrap()
                .capture_and_verify(&mut reader)
                .map(|_| ())
                .map_err(|error| error.id())
        }));
    }
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .copied()
            .collect::<Vec<_>>(),
        vec![HostLifecycleErrorId::SessionStateRejected]
    );
    assert_eq!(fixture.tree(), before_tree);
}

#[test]
fn external_callers_cannot_capture_construct_clone_or_deserialize_observation_frames() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = std::path::PathBuf::from("/tmp").join(format!(
        "hul-host-observation-privacy-probe-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/bin")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "host-lifecycle-privacy-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
ultragoal = {{ path = "{}" }}
serde = {{ version = "1.0.228", features = ["derive"] }}
serde_json = "1.0.150"
sha2 = "0.10.9"
"#,
            manifest.display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        format!(
            r#"pub mod distribution {{
    pub use ultragoal::distribution::*;
}}
pub mod plugin_product {{
    pub mod distribution_adapter {{
        pub use ultragoal::plugin_product::distribution_adapter::*;
    }}
    pub mod lifecycle {{
        pub use ultragoal::plugin_product::lifecycle::*;
    }}
}}
#[path = "{}"]
pub mod host_lifecycle;
"#,
            manifest
                .join("src/plugin_product/host_lifecycle/mod.rs")
                .display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/bin/forge.rs"),
        r#"use host_lifecycle_privacy_probe::host_lifecycle::{
    capture_host_observations, HostLifecycleSession, HostObservationExpectations,
    HostObservationFrame, HostObservationTransactionRequest, HostSurfaceReader,
};

fn require_clone<T: Clone>() {}
fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}

fn try_two_step<R: HostSurfaceReader>(session: &mut HostLifecycleSession, reader: &mut R) {
    let frame = session.capture_observations(reader).unwrap();
    let _ = session.verify_observations(frame);
}

fn main() {
    let _ = capture_host_observations;
    let _: Option<HostObservationExpectations<'static>> = None;
    require_clone::<HostObservationFrame>();
    require_deserializable::<HostObservationFrame>();
    let _ = HostObservationTransactionRequest::issue;
    require_clone::<HostObservationTransactionRequest>();
    require_deserializable::<HostObservationTransactionRequest>();
}
"#,
    )
    .unwrap();
    let output =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args([
                "check",
                "--offline",
                "--quiet",
                "--jobs",
                "16",
                "--bin",
                "forge",
            ])
            .env(
                "CARGO_TARGET_DIR",
                "/tmp/hul-plugin-host-lifecycle-044-rework6-compile-red",
            )
            .current_dir(&root)
            .output()
            .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "raw observation forgery compiled");
    for expected in [
        "capture_host_observations",
        "capture_observations",
        "verify_observations",
        "HostObservationExpectations",
        "HostObservationFrame",
        "HostObservationTransactionRequest",
        "issue",
        "Clone",
        "Deserialize",
    ] {
        assert!(
            stderr.contains(expected),
            "unexpected compile failure missing {expected}: {stderr}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
