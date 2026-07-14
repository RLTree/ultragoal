use super::*;

#[cfg(test)]
pub(crate) fn duplicate_authorization_for_test<E: RepositoryFitPermitEffects>(
    permit: &RepositoryFitApplyPermit,
    request: &OpaqueFitApplyRequest,
    effects: E,
) -> (RepositoryFitApplyPermit, RepositoryFitMutationLease<E>) {
    let permit_target = capture_target_descriptor_chain(&permit.target_chain.root_path, request)
        .expect("duplicate test authority requires the exact live target chain");
    assert_eq!(permit_target.snapshot, permit.target_prestate);
    let lease_target = capture_target_descriptor_chain(&permit.target_chain.root_path, request)
        .expect("duplicate test lease requires the exact live target chain");
    assert_eq!(lease_target.snapshot, permit.target_prestate);
    (
        RepositoryFitApplyPermit {
            permit_id: permit.permit_id.clone(),
            binding: permit.binding.clone(),
            issued_tick: permit.issued_tick,
            expires_tick: permit.expires_tick,
            nonce_sha256: permit.nonce_sha256.clone(),
            authority: Arc::clone(&permit.authority),
            seal: Arc::clone(&permit.seal),
            target_prestate: permit.target_prestate.clone(),
            target_chain: permit_target.chain,
            protected_prestate: permit.protected_prestate.clone(),
        },
        RepositoryFitMutationLease {
            binding_id: permit.binding.plan_record_sha256.clone(),
            authority: Arc::clone(&permit.authority),
            seal: Arc::clone(&permit.seal),
            effects: ScopedEffects::new(
                effects,
                request,
                &permit.target_chain.root_path,
                permit.target_prestate.clone(),
                lease_target,
            ),
        },
    )
}

#[cfg(test)]
pub(crate) fn permit_seal_stage_for_test(request: &OpaqueFitApplyRequest) -> u8 {
    request.seal.stage_for_test()
}

#[cfg(test)]
pub(crate) fn before_final_green_observation_for_test(action: impl FnOnce() + 'static) {
    BEFORE_FINAL_GREEN_OBSERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "a final-green test action is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn before_postflight_observation_for_test(action: impl FnOnce() + 'static) {
    BEFORE_POSTFLIGHT_OBSERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "a postflight test action is already armed");
    });
}

#[cfg(test)]
pub(crate) fn target_capture_hook_for_test(
    phase: TargetCapturePhase,
    path: impl Into<String>,
    action: impl FnOnce() + 'static,
) {
    TARGET_CAPTURE_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(TargetCaptureHook {
            phase,
            path: path.into(),
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a target-capture test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn assert_target_capture_hook_consumed_for_test() {
    TARGET_CAPTURE_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed target-capture test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(crate) fn protected_capture_hook_for_test(
    boundary: ProtectedCaptureBoundary,
    phase: ProtectedCapturePhase,
    path: impl Into<Vec<u8>>,
    action: impl FnOnce() + 'static,
) {
    PROTECTED_CAPTURE_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(ProtectedCaptureHook {
            boundary,
            phase,
            path: path.into(),
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a protected-capture test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn assert_protected_capture_hook_consumed_for_test() {
    PROTECTED_CAPTURE_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed protected-capture test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(crate) fn reconciliation_target_hook_for_test(
    phase: ReconciliationTargetPhase,
    action: impl FnOnce() + 'static,
) {
    RECONCILIATION_TARGET_HOOK.with(|slot| {
        let prior = slot.borrow_mut().replace(ReconciliationTargetHook {
            phase,
            action: Box::new(action),
        });
        assert!(
            prior.is_none(),
            "a reconciliation-target test hook is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn assert_reconciliation_target_hook_consumed_for_test() {
    RECONCILIATION_TARGET_HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "the armed reconciliation-target test hook was not reached"
        );
    });
}

#[cfg(test)]
pub(crate) fn scope_violation_for_test<E: RepositoryFitPermitEffects>(
    root: &Path,
    request: &OpaqueFitApplyRequest,
    effects: E,
) -> (FitErrorId, bool) {
    let target = capture_target_descriptor_chain(root, request).unwrap();
    let target_prestate = target.snapshot.clone();
    let mut scoped = ScopedEffects::new(effects, request, root, target_prestate, target);
    let path = CanonicalPath::parse("private/undeclared-scope-probe")
        .expect("fixed test probe path is canonical");
    let failure = scoped
        .compare_exchange(&path, &ExpectedContent::Absent, Some(b"probe"))
        .expect_err("an undeclared test mutation must be refused");
    (failure.id(), scoped.scope_violation())
}
