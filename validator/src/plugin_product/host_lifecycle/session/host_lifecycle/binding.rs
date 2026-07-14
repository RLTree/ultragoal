impl HostLifecycleSession {
    pub fn bind(request: HostLifecycleBindRequest<'_>) -> Result<Self, HostLifecycleError> {
        let HostLifecycleBindRequest {
            root,
            package_plan,
            package,
            lifecycle,
            host,
            marketplace_plan,
            host_scope,
        } = request;
        if package_plan.context_id() != package.context_id()
            || package_plan.candidate_id() != package.candidate_id()
            || package_plan.version() != package.identity().source().version()
            || marketplace_plan.package() != package.identity()
        {
            return Err(invalid());
        }
        let host_scope =
            BoundHostScope::bind(package.identity(), &host, &marketplace_plan, host_scope)?;
        let binding = JourneyBinding::new(
            package.identity().clone(),
            &host,
            host_scope.authority().marketplace(),
        )
        .map_err(|_| invalid())?;
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
}
