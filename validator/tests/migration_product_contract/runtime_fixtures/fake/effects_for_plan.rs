impl FakeEffects {
    pub(crate) fn for_plan(plan: &ProductMigrationPlan) -> Self {
        let authorities = plan
            .effects()
            .iter()
            .map(|effect| (effect.effect_id().to_owned(), effect.before().clone()))
            .collect();
        let effect_permit_sha256 = plan
            .effects()
            .iter()
            .map(|effect| (effect.effect_id().to_owned(), None))
            .collect();
        Self {
            state: Arc::new(Mutex::new(EffectState {
                authorities,
                effect_permit_sha256,
                ..EffectState::default()
            })),
        }
    }

    pub(crate) fn reject(&self, effect_id: &str) {
        self.state.lock().unwrap().reject_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn reject_after_effect(&self, effect_id: &str) {
        self.state.lock().unwrap().reject_after_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn ambiguous(&self, effect_id: &str) {
        self.state.lock().unwrap().ambiguous_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn substitute_effect_permit(&self, effect_id: &str, permit: Option<String>) {
        self.state
            .lock()
            .unwrap()
            .effect_permit_sha256
            .insert(effect_id.to_owned(), permit);
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        let state = self.state.lock().unwrap();
        (state.apply_count, state.rollback_count)
    }

    pub(crate) fn compatibility_prerequisite_inputs(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .compatibility_prerequisite_inputs
            .clone()
    }

    pub(crate) fn authority(&self, effect_id: &str) -> AuthoritySnapshot {
        self.state
            .lock()
            .unwrap()
            .authorities
            .get(effect_id)
            .unwrap()
            .clone()
    }

    fn observation(
        authority: AuthoritySnapshot,
        effect_permit_sha256: Option<String>,
    ) -> Result<EffectObservation, EffectFault> {
        EffectObservation::live(authority, sha('c'), effect_permit_sha256)
            .map_err(|_| EffectFault::ambiguous("test-observation-invalid"))
    }
}

impl ConfinedMigrationEffect for FakeEffects {
    fn observe(
        &mut self,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        let state = self.state.lock().unwrap();
        let authority = state
            .authorities
            .get(effect.effect_id())
            .cloned()
            .ok_or_else(|| EffectFault::ambiguous("test-effect-unknown"))?;
        let permit = state
            .effect_permit_sha256
            .get(effect.effect_id())
            .cloned()
            .flatten();
        Self::observation(authority, permit)
    }

    fn apply(
        &mut self,
        _operation_id: &str,
        effect: &PlannedMigrationEffect,
        compatibility_effect_permit_sha256: Option<&str>,
    ) -> Result<EffectObservation, EffectFault> {
        let mut state = self.state.lock().unwrap();
        if state.ambiguous_effect_id.as_deref() == Some(effect.effect_id()) {
            return Err(EffectFault::ambiguous("test-effect-ambiguous"));
        }
        if state.reject_effect_id.as_deref() == Some(effect.effect_id()) {
            return Err(EffectFault::rejected("test-effect-rejected"));
        }
        if state.reject_after_effect_id.as_deref() == Some(effect.effect_id()) {
            state.apply_count += 1;
            state
                .authorities
                .insert(effect.effect_id().to_owned(), effect.after().clone());
            state.effect_permit_sha256.insert(
                effect.effect_id().to_owned(),
                compatibility_effect_permit_sha256.map(ToOwned::to_owned),
            );
            return Err(EffectFault::rejected("test-effect-rejected-after-effect"));
        }
        state.apply_count += 1;
        if let Some(prerequisites_sha256) = effect.compatibility_prerequisites_sha256() {
            state
                .compatibility_prerequisite_inputs
                .push(prerequisites_sha256.to_owned());
        }
        state
            .authorities
            .insert(effect.effect_id().to_owned(), effect.after().clone());
        state.effect_permit_sha256.insert(
            effect.effect_id().to_owned(),
            compatibility_effect_permit_sha256.map(ToOwned::to_owned),
        );
        Self::observation(
            effect.after().clone(),
            compatibility_effect_permit_sha256.map(ToOwned::to_owned),
        )
    }

    fn rollback(
        &mut self,
        _operation_id: &str,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        let mut state = self.state.lock().unwrap();
        state.rollback_count += 1;
        state
            .authorities
            .insert(effect.effect_id().to_owned(), effect.before().clone());
        state
            .effect_permit_sha256
            .insert(effect.effect_id().to_owned(), None);
        Self::observation(effect.before().clone(), None)
    }
}
