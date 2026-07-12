use super::capability_gate::HostEffectCapabilityGate;
use super::effect_request::{
    ExternalHostEffectRequest, HostScopeAuthority, PreparedExternalHostEffect,
};
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::SessionIssuance;
use super::model::{
    HostLayer, HostLayerReport, HostLayerVerdict, HostLifecyclePhase, HostLifecycleReport,
};
use super::observation::{
    BoundHostObservationTransaction, HostObservationFrame, HostObservationTransactionRequest,
    HostSurfaceReader, HostSurfaceTransaction, HostSurfaceTransactionError,
};
use super::scope::BoundHostScope;
use super::verify::verify_host_identity_chain;
use crate::distribution::{
    ConfinedRoot, HostCapabilityDeclaration, JourneyBinding, MarketplacePlan, PackagePlan,
    PackageSnapshot,
};
use crate::plugin_product::distribution_adapter::DistributionLifecycleOperation;
use crate::plugin_product::lifecycle::{
    ApplyDisposition, ApplyReport, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub struct HostLifecycleSession {
    package: PackageSnapshot,
    lifecycle: LifecyclePlan,
    operation: DistributionLifecycleOperation,
    binding: JourneyBinding,
    marketplace_plan: MarketplacePlan,
    host_scope: Arc<BoundHostScope>,
    capability_gate: Arc<HostEffectCapabilityGate>,
    issuance: Arc<SessionIssuance>,
    external_effect_request: Option<ExternalHostEffectRequest>,
    external_effect_request_sha256: Option<String>,
}

impl HostLifecycleSession {
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        root: ConfinedRoot,
        package_plan: &PackagePlan,
        package: &PackageSnapshot,
        lifecycle: LifecyclePlan,
        host: HostCapabilityDeclaration,
        marketplace_plan: MarketplacePlan,
        host_scope: HostScopeAuthority,
    ) -> Result<Self, HostLifecycleError> {
        if package_plan.context_id() != package.context_id()
            || package_plan.candidate_id() != package.candidate_id()
            || package_plan.version() != package.identity().source().version()
            || marketplace_plan.package() != package.identity()
        {
            return Err(invalid());
        }
        let binding =
            JourneyBinding::new(package.identity().clone(), &host).map_err(|_| invalid())?;
        let host_scope =
            BoundHostScope::bind(package.identity(), &host, &marketplace_plan, host_scope)?;
        let operation =
            DistributionLifecycleOperation::bind(root, package_plan, package, &lifecycle)
                .map_err(|_| invalid())?;
        let capability_gate =
            HostEffectCapabilityGate::issue(lifecycle.intent, Arc::clone(&host_scope))?;
        let issuance = SessionIssuance::issue(&lifecycle, &binding)?;
        let external_effect_request = ExternalHostEffectRequest::for_intent(
            package.identity(),
            &binding,
            &lifecycle,
            Arc::clone(&issuance),
            Arc::clone(&capability_gate),
            Arc::clone(&host_scope),
            lifecycle.intent,
        )?;
        let external_effect_request_sha256 = external_effect_request
            .as_ref()
            .map(|request| request.request_sha256().to_owned());
        Ok(Self {
            package: package.clone(),
            lifecycle,
            operation,
            binding,
            marketplace_plan,
            host_scope,
            capability_gate,
            issuance,
            external_effect_request,
            external_effect_request_sha256,
        })
    }

    pub fn apply_confined(
        &mut self,
        observed: &LifecycleState,
    ) -> Result<ApplyReport, HostLifecycleError> {
        self.issuance.begin_apply()?;
        self.require_host_effect_capability()?;
        match self.operation.apply(observed, &self.lifecycle) {
            Ok(report) => {
                let applied = report.disposition == ApplyDisposition::Applied
                    && report.state == self.lifecycle.expected_after;
                self.issuance
                    .finish_apply(applied, self.external_effect_request.is_some())?;
                Ok(report)
            }
            Err(_) => {
                self.issuance.reject_apply();
                Err(HostLifecycleError::new(
                    HostLifecycleErrorId::LifecycleRejected,
                ))
            }
        }
    }

    pub fn capture_and_verify(
        &mut self,
        reader: &mut impl HostSurfaceReader,
    ) -> Result<HostLifecycleReport, HostLifecycleError> {
        self.require_observation_preflight()?;
        let request = HostObservationTransactionRequest::issue(
            self.binding.binding_sha256(),
            &self.host_scope,
            &self.issuance,
        );
        reader.with_transaction(&request, |transaction| {
            let transaction = transaction.map_err(transaction_error)?;
            self.capture_and_verify_transaction(transaction, &request)
        })
    }

    fn capture_and_verify_transaction(
        &mut self,
        transaction: &mut dyn HostSurfaceTransaction,
        request: &HostObservationTransactionRequest,
    ) -> Result<HostLifecycleReport, HostLifecycleError> {
        self.require_host_effect_capability()?;
        if let Err(error) = self.host_scope.revalidate() {
            self.issuance.reject();
            return Err(error);
        }
        self.issuance.require_observation_ready()?;
        let mut transaction = BoundHostObservationTransaction::bind(
            transaction,
            request,
            &self.host_scope,
            &self.issuance,
        )?;
        let captured_state = self.observe_expected_state()?;
        let sealed = self.capture_observations(&mut transaction, request)?;
        self.require_host_effect_capability()?;
        transaction.require_current(request)?;
        sealed.require_exact_session(&self.binding, &self.host_scope, &self.issuance)?;
        let between_state = self.observe_expected_state()?;
        if between_state != captured_state {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }

        let fresh = self.capture_observations(&mut transaction, request)?;
        self.require_host_effect_capability()?;
        transaction.require_current(request)?;
        sealed.require_exact_session(&self.binding, &self.host_scope, &self.issuance)?;
        fresh.require_exact_session(&self.binding, &self.host_scope, &self.issuance)?;
        let final_state = self.observe_expected_state()?;
        if final_state != captured_state || !sealed.same_observation(&fresh) {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }
        if transaction.transaction_provenance_sha256() != sealed.reader_provenance_sha256()
            || transaction.transaction_provenance_sha256() != fresh.reader_provenance_sha256()
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }
        self.require_host_effect_capability()?;
        transaction.require_current(request)?;
        sealed.require_exact_session(&self.binding, &self.host_scope, &self.issuance)?;
        fresh.require_exact_session(&self.binding, &self.host_scope, &self.issuance)?;
        let report = self.verify_observations(sealed, final_state)?;
        transaction.require_current(request)?;
        transaction.commit(request, report)
    }

    fn capture_observations(
        &self,
        transaction: &mut BoundHostObservationTransaction<'_>,
        request: &HostObservationTransactionRequest,
    ) -> Result<HostObservationFrame, HostLifecycleError> {
        self.require_host_effect_capability()?;
        self.issuance.require_observation_ready()?;
        let result = transaction.capture(
            &self.marketplace_plan,
            &self.binding,
            self.lifecycle.intent == LifecycleIntent::UninstallTeardown,
            request,
        );
        if result.as_ref().is_err_and(|error| {
            matches!(
                error.id(),
                HostLifecycleErrorId::HostScopeRejected | HostLifecycleErrorId::IdentityMismatch
            )
        }) {
            self.issuance.reject();
        }
        result
    }

    #[cfg(test)]
    fn capture_observations_for_test(
        &self,
        reader: &mut impl HostSurfaceReader,
    ) -> Result<HostObservationFrame, HostLifecycleError> {
        self.require_observation_preflight()?;
        let request = HostObservationTransactionRequest::issue(
            self.binding.binding_sha256(),
            &self.host_scope,
            &self.issuance,
        );
        reader.with_transaction(&request, |transaction| {
            let transaction = transaction.map_err(transaction_error)?;
            let mut transaction = BoundHostObservationTransaction::bind(
                transaction,
                &request,
                &self.host_scope,
                &self.issuance,
            )?;
            self.capture_observations(&mut transaction, &request)
        })
    }

    fn verify_observations(
        &mut self,
        frame: HostObservationFrame,
        state: LifecycleState,
    ) -> Result<HostLifecycleReport, HostLifecycleError> {
        self.require_host_effect_capability()?;
        self.issuance.require_observation_ready()?;
        if let Err(error) =
            frame.require_exact_session(&self.binding, &self.host_scope, &self.issuance)
        {
            self.issuance.reject();
            return Err(error);
        }
        if state != self.lifecycle.expected_after {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::IdentityMismatch,
            ));
        }
        let package_row = HostLayerReport::new(
            HostLayer::Package,
            HostLayerVerdict::Verified,
            Some(self.package.package_sha256().to_owned()),
        );
        let installed_sha256 = digest(
            &serde_json::to_vec(&state.installed)
                .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?,
        );
        let installed_row = HostLayerReport::new(
            HostLayer::Installed,
            if state.installed.is_some() {
                HostLayerVerdict::Verified
            } else {
                HostLayerVerdict::ObservedAbsent
            },
            state.installed.as_ref().map(|_| installed_sha256.clone()),
        );
        let layers = ordered_layers(package_row, installed_row, frame.layers())?;
        let identity_chain_sha256 =
            identity_chain(&self.package, &self.binding, &installed_sha256, &frame)?;
        let phase = if self.lifecycle.intent == LifecycleIntent::UninstallTeardown {
            if state.installed.is_none()
                && state.cache.is_none()
                && frame.layer(HostLayer::Cache).verdict() == HostLayerVerdict::ObservedAbsent
                && frame.layer(HostLayer::Discovery).verdict() == HostLayerVerdict::ObservedAbsent
                && frame.layer(HostLayer::Runtime).verdict() == HostLayerVerdict::ObservedAbsent
            {
                HostLifecyclePhase::TeardownObserved
            } else {
                HostLifecyclePhase::AwaitingSupportedHostObservation
            }
        } else if layers
            .iter()
            .all(|row| row.verdict() == HostLayerVerdict::Verified)
        {
            HostLifecyclePhase::Observed
        } else {
            HostLifecyclePhase::AwaitingSupportedHostObservation
        };
        let report = HostLifecycleReport::new(
            self.lifecycle.intent,
            phase,
            state.clone(),
            layers,
            self.external_effect_request_sha256.clone(),
            self.external_effect_request_sha256.is_some(),
            identity_chain_sha256,
        );
        self.require_host_effect_capability()?;
        if let Err(error) =
            frame.require_exact_session(&self.binding, &self.host_scope, &self.issuance)
        {
            self.issuance.reject();
            return Err(error);
        }
        let closing_state = self.operation.observe_state().map_err(|_| invalid())?;
        if closing_state != state {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }
        Ok(report)
    }

    pub fn take_external_effect_request(
        &mut self,
    ) -> Result<ExternalHostEffectRequest, HostLifecycleError> {
        if self.external_effect_request_sha256.is_none() {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ExternalEffectNotEligible,
            ));
        }
        self.require_host_effect_capability()?;
        self.issuance.issue_request()?;
        self.external_effect_request
            .take()
            .ok_or_else(|| HostLifecycleError::new(HostLifecycleErrorId::ExternalEffectReplayed))
    }

    pub fn consume_external_effect_request(
        &self,
        request: ExternalHostEffectRequest,
    ) -> Result<PreparedExternalHostEffect, HostLifecycleError> {
        request.consume_for(&self.issuance, &self.capability_gate, &self.host_scope)
    }

    #[cfg(test)]
    pub(crate) fn session_issuance_sha256(&self) -> &str {
        self.issuance.issuance_sha256()
    }

    #[cfg(test)]
    pub(crate) fn host_scope_sha256(&self) -> &str {
        self.host_scope.scope_sha256()
    }

    pub fn binding(&self) -> &JourneyBinding {
        &self.binding
    }

    pub fn plan(&self) -> &LifecyclePlan {
        &self.lifecycle
    }

    pub fn observe_confined_state(&self) -> Result<LifecycleState, HostLifecycleError> {
        self.operation.observe_state().map_err(|_| invalid())
    }

    fn require_host_effect_capability(&self) -> Result<(), HostLifecycleError> {
        if let Err(error) = self.capability_gate.require_supported() {
            self.issuance.reject();
            return Err(error);
        }
        Ok(())
    }

    fn observe_expected_state(&self) -> Result<LifecycleState, HostLifecycleError> {
        let state = self.operation.observe_state().map_err(|_| invalid())?;
        if state != self.lifecycle.expected_after {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::IdentityMismatch,
            ));
        }
        Ok(state)
    }

    fn require_observation_preflight(&self) -> Result<LifecycleState, HostLifecycleError> {
        self.require_host_effect_capability()?;
        if let Err(error) = self.host_scope.revalidate() {
            self.issuance.reject();
            return Err(error);
        }
        self.issuance.require_observation_ready()?;
        let observed = self.observe_expected_state()?;

        self.require_host_effect_capability()?;
        if let Err(error) = self.host_scope.revalidate() {
            self.issuance.reject();
            return Err(error);
        }
        self.issuance.require_observation_ready()?;
        if self.observe_expected_state()? != observed {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ObservationChanged,
            ));
        }
        Ok(observed)
    }
}

fn ordered_layers(
    package: HostLayerReport,
    installed: HostLayerReport,
    observed: &[HostLayerReport],
) -> Result<Vec<HostLayerReport>, HostLifecycleError> {
    let mut rows = vec![package];
    for layer in HostLayer::ALL.into_iter().skip(1) {
        if layer == HostLayer::Installed {
            rows.push(installed.clone());
            continue;
        }
        let row = observed
            .iter()
            .find(|row| row.layer() == layer)
            .ok_or_else(|| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?;
        rows.push(row.clone());
    }
    if rows.len() != HostLayer::ALL.len() {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::IdentityMismatch,
        ));
    }
    Ok(rows)
}

fn identity_chain(
    package: &PackageSnapshot,
    binding: &JourneyBinding,
    installed_sha256: &str,
    frame: &HostObservationFrame,
) -> Result<Option<String>, HostLifecycleError> {
    let Some(marketplace) = frame.marketplace.as_ref() else {
        return Ok(None);
    };
    let Some(cache) = frame.cache.as_ref() else {
        return Ok(None);
    };
    let Some(discovery) = frame.discovery.as_ref() else {
        return Ok(None);
    };
    let Some(runtime) = frame.runtime.as_ref() else {
        return Ok(None);
    };
    let Some(marketplace_sha256) = marketplace.catalog_sha256() else {
        return Ok(None);
    };
    let Some(discovery_sha256) = discovery.observation_sha256() else {
        return Ok(None);
    };
    let Some(runtime_sha256) = runtime.output_sha256() else {
        return Ok(None);
    };
    verify_host_identity_chain(
        package.identity(),
        binding,
        marketplace_sha256,
        installed_sha256,
        cache.observation_sha256(),
        discovery_sha256,
        runtime_sha256,
    )
    .map(Some)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding)
}

fn transaction_error(error: HostSurfaceTransactionError) -> HostLifecycleError {
    HostLifecycleError::new(match error {
        HostSurfaceTransactionError::Unsupported => {
            HostLifecycleErrorId::ObservationTransactionUnsupported
        }
        HostSurfaceTransactionError::Failed => HostLifecycleErrorId::ObservationUnavailable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{Fixture, Reader, installed, lifecycle, request};

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
        let mut personal_owner = HostLifecycleSession::bind(
            fixture.confined(),
            &bundle.plan,
            &bundle.snapshot,
            repeat(),
            host.clone(),
            crate::support::marketplace_plan(&bundle),
            personal_scope.clone(),
        )
        .unwrap();
        let mut personal_sibling = HostLifecycleSession::bind(
            fixture.confined(),
            &bundle.plan,
            &bundle.snapshot,
            repeat(),
            host.clone(),
            crate::support::marketplace_plan(&bundle),
            personal_scope,
        )
        .unwrap();
        let mut repository = HostLifecycleSession::bind(
            fixture.confined(),
            &bundle.plan,
            &bundle.snapshot,
            repeat(),
            host.clone(),
            crate::support::marketplace_plan(&bundle),
            repository_scope,
        )
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
}
