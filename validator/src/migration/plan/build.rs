impl MigrationPlan {
    pub(crate) fn build<A: ReplacementEvidenceAuthority>(
        inventory: &MigrationInventory,
        mut routes: Vec<CompatibilityRoute>,
        evidence: Vec<ReplacementEvidence>,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        inventory.validate()?;
        if routes.is_empty() || routes.len() > MAX_ROUTES || routes.len() != evidence.len() {
            return Err(MigrationError::new(
                "migration-route-evidence-count-invalid",
            ));
        }
        routes.sort_by(|left, right| left.route_id.cmp(&right.route_id));
        let surfaces = inventory
            .surfaces
            .iter()
            .map(|surface| (surface.stable_id.as_str(), surface))
            .collect::<BTreeMap<_, _>>();
        let mut evidence = evidence
            .into_iter()
            .map(|row| (row.source_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        if evidence.len() != routes.len() {
            return Err(MigrationError::new(
                "migration-duplicate-replacement-evidence",
            ));
        }
        let mut route_ids = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        let mut targets = Vec::with_capacity(routes.len());
        let mut replacement_ledger = PlanReplacementLedger::new();
        for route in &routes {
            route.validate()?;
            if !route_ids.insert(route.route_id.as_str()) {
                return Err(MigrationError::new("migration-duplicate-route"));
            }
            if !source_ids.insert(route.source_id.as_str()) {
                return Err(MigrationError::new("migration-ambiguous-source-route"));
            }
            let source = surfaces
                .get(route.source_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-route-source-unknown"))?;
            let canonical = surfaces
                .get(route.canonical_target_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-route-target-unknown"))?;
            if canonical.status == SurfaceStatus::Retired
                || canonical.status == SurfaceStatus::ContextOnly
            {
                return Err(MigrationError::new("migration-canonical-target-inactive"));
            }
            let replacement = evidence
                .remove(route.source_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-replacement-evidence-missing"))?;
            if replacement.canonical_target_id != route.canonical_target_id {
                return Err(MigrationError::new(
                    "migration-replacement-evidence-conflict",
                ));
            }
            let replacement = replacement.consume(inventory, route, authority)?;
            let target_id = format!("retire-{}", route.source_id);
            let replacement_summary_sha256 = replacement_summary_commitment(&replacement);
            let target = RetirementTarget {
                target_id: target_id.clone(),
                route_id: route.route_id.clone(),
                source_id: route.source_id.clone(),
                canonical_target_id: route.canonical_target_id.clone(),
                source_status: source.status,
                active_readers: source.active_readers.clone(),
                active_writers: source.active_writers.clone(),
                public_routes: source.public_routes.clone(),
                generated_outputs: source.generated_outputs.clone(),
                observed_invocations: route.observed_invocations,
                compatibility_window_complete: route.compatibility_window_complete,
                owner_id: route.owner_id.clone(),
                replacement_summary_sha256,
            };
            replacement_ledger.insert(target_id, replacement)?;
            targets.push(target);
        }
        if !evidence.is_empty() {
            return Err(MigrationError::new(
                "migration-replacement-evidence-unmatched",
            ));
        }
        targets.sort_by(|left, right| left.target_id.cmp(&right.target_id));
        let plan_sha256 = plan_digest(inventory, &routes, &targets);
        let plan = Self {
            live_context_id: inventory.live_context_id.clone(),
            candidate_id: inventory.candidate_id.clone(),
            catalog_id: inventory.catalog_id.clone(),
            read_session_id: inventory.read_session_id.clone(),
            inventory_sha256: inventory.inventory_sha256.clone(),
            plan_sha256,
            routes,
            targets,
            replacement_ledger,
        };
        for target in &plan.targets {
            let route = plan
                .routes
                .iter()
                .find(|route| route.route_id == target.route_id)
                .ok_or_else(|| MigrationError::new("migration-replacement-route-missing"))?;
            plan.replacement_evidence(target)
                .ok_or_else(|| {
                    MigrationError::new("migration-private-replacement-evidence-missing")
                })?
                .validate_with_authority(inventory, route, authority)?;
            let binding = ReplacementLedgerBinding::issue(&plan, target, inventory)?;
            authority.bind_plan_target(&binding)?;
            if !authority.verify_plan_target_consumed(&binding)
                || plan.replacement_evidence(target).is_none_or(|evidence| {
                    evidence
                        .validate_with_authority(inventory, route, authority)
                        .is_err()
                })
                || !authority.verify_plan_target_consumed(&binding)
            {
                return Err(MigrationError::new(
                    "migration-replacement-ledger-binding-refused",
                ));
            }
        }
        Ok(plan)
    }

    pub fn verify_current(&self, current: &MigrationInventory) -> Result<(), MigrationError> {
        current.validate()?;
        if self.live_context_id != current.live_context_id
            || self.candidate_id != current.candidate_id
            || self.catalog_id != current.catalog_id
            || self.read_session_id != current.read_session_id
            || self.inventory_sha256 != current.inventory_sha256
            || self.plan_sha256 != plan_digest(current, &self.routes, &self.targets)
            || !self.private_replacement_ledger_is_current()
        {
            return Err(MigrationError::new("migration-plan-stale"));
        }
        Ok(())
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn targets(&self) -> &[RetirementTarget] {
        &self.targets
    }

    /// Safe inspection DTO. It deliberately omits the private consumed
    /// replacement ledger and cannot be converted back into migration
    /// authority.
    pub fn projection(&self) -> MigrationPlanProjection {
        let targets = self
            .targets
            .iter()
            .map(RetirementTarget::projection)
            .collect::<Vec<_>>();
        let context_commitment_sha256 = digest(
            format!(
                "migration-plan-context-v1|{}|{}|{}|{}|{}",
                self.live_context_id,
                self.candidate_id,
                self.catalog_id,
                self.read_session_id,
                self.inventory_sha256,
            )
            .as_bytes(),
        );
        let projection_sha256 = migration_plan_projection_digest(
            &self.plan_sha256,
            &context_commitment_sha256,
            self.routes.len(),
            &targets,
        );
        MigrationPlanProjection {
            schema_version: "MigrationPlanProjection-v1".to_owned(),
            plan_sha256: self.plan_sha256.clone(),
            context_commitment_sha256,
            route_count: self.routes.len(),
            target_count: targets.len(),
            targets,
            projection_sha256,
        }
    }
}
