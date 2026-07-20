impl RecoveryAuthorizationSeal {
    pub(super) fn issue(
        plan_id: &str,
        prior: &LifecycleState,
        expected_current: &LifecycleState,
        action_state: LifecycleActionAuthority,
        recovery_state: RecoveryStateAuthority,
    ) -> Self {
        Self::Sealed(Box::new(RecoveryAuthorizationSealData {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            plan_id: plan_id.to_owned(),
            prior: prior.clone(),
            expected_current: expected_current.clone(),
            action_state,
            recovery_state,
        }))
    }

    pub(super) fn authorized_restore(&self) -> Option<(&str, &LifecycleState, &LifecycleState)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed(data) => Some((&data.plan_id, &data.prior, &data.expected_current)),
        }
    }

    pub(super) fn consume_for_observed(
        &self,
        observed_state: &LifecycleState,
    ) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        let recovery_state = data
            .recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        loop {
            let action_state_value =
                LifecycleActionState::decode(data.action_state.load(AtomicOrdering::Acquire))?;
            match action_state_value {
                LifecycleActionState::Planned
                | LifecycleActionState::Transferred
                | LifecycleActionState::Applying => {
                    return Err(LifecycleError::RecoveryUnavailable);
                }
                LifecycleActionState::RecoveryAvailable => {
                    if recovery_state.as_ref() != Some(observed_state) {
                        return Err(LifecycleError::StaleRecoveryToken);
                    }
                    if data
                        .action_state
                        .compare_exchange_weak(
                            LifecycleActionState::RecoveryAvailable as u8,
                            LifecycleActionState::Recovering as u8,
                            AtomicOrdering::AcqRel,
                            AtomicOrdering::Acquire,
                        )
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                LifecycleActionState::Recovering | LifecycleActionState::Closed => {
                    return Err(LifecycleError::ReplayedRecoveryToken);
                }
            }
        }
    }

    pub(super) fn close(&self) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        data.action_state
            .compare_exchange(
                LifecycleActionState::Recovering as u8,
                LifecycleActionState::Closed as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::InvalidTransition)
    }
}

impl fmt::Debug for RecoveryAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed(data) => formatter
                .debug_struct("Sealed")
                .field("issuance_id", &data.issuance_id)
                .field("plan_id", &data.plan_id)
                .field("prior", &data.prior)
                .field("expected_current", &data.expected_current)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for RecoveryAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (Self::Sealed(left), Self::Sealed(right)) => {
                left.issuance_id == right.issuance_id
                    && left.plan_id == right.plan_id
                    && left.prior == right.prior
                    && left.expected_current == right.expected_current
            }
            _ => false,
        }
    }
}

impl Eq for RecoveryAuthorizationSeal {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    AuthorizationRequired,
    DowngradeAuthorizationRequired,
    ExpectedPriorMismatch,
    InconsistentState,
    InvalidDigest,
    InvalidTransition,
    InvalidVersion,
    MissingPriorAuthority,
    ReadEffectFailed {
        effect: LifecycleEffect,
        causal_error: String,
    },
    RecoveryObservationFailed(String),
    RecoveryFailed(String),
    RecoveryStateMismatch,
    RecoveryUnavailable,
    ReplayedPlan,
    ReplayedRecoveryToken,
    StalePlan,
    StaleRecoveryToken,
    UnsealedPlan,
    UnsealedRecoveryToken,
    VerificationFailed,
}

pub trait LifecycleEffectAdapter {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        expected_after: &LifecycleState,
    ) -> Result<(), String>;
    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String>;
    fn observe_state(&self) -> Result<LifecycleState, String>;
}
