impl HostLifecycleSession {
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
