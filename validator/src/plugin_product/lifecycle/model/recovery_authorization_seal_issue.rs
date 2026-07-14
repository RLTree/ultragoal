impl RecoveryAuthorizationSeal {
    pub(super) fn issue(
        plan_id: &str,
        prior: &LifecycleState,
        expected_current: &LifecycleState,
        action_state: Arc<AtomicU8>,
        recovery_state: Arc<Mutex<Option<LifecycleState>>>,
    ) -> Self {
        Self::Sealed {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            plan_id: plan_id.to_owned(),
            prior: prior.clone(),
            expected_current: expected_current.clone(),
            action_state,
            recovery_state,
        }
    }

    pub(super) fn authorized_restore(&self) -> Option<(&str, &LifecycleState, &LifecycleState)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed {
                plan_id,
                prior,
                expected_current,
                ..
            } => Some((plan_id, prior, expected_current)),
        }
    }

    pub(super) fn consume_for_observed(
        &self,
        observed_state: &LifecycleState,
    ) -> Result<(), LifecycleError> {
        let Self::Sealed {
            action_state,
            recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        let recovery_state = recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        loop {
            let action_state_value =
                LifecycleActionState::decode(action_state.load(AtomicOrdering::Acquire))?;
            match action_state_value {
                LifecycleActionState::Planned | LifecycleActionState::Applying => {
                    return Err(LifecycleError::RecoveryUnavailable);
                }
                LifecycleActionState::RecoveryAvailable => {
                    if recovery_state.as_ref() != Some(observed_state) {
                        return Err(LifecycleError::StaleRecoveryToken);
                    }
                    if action_state
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
        let Self::Sealed { action_state, .. } = self else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        action_state
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

impl Default for RecoveryAuthorizationSeal {
    fn default() -> Self {
        Self::Unsealed
    }
}

impl fmt::Debug for RecoveryAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed {
                issuance_id,
                plan_id,
                prior,
                expected_current,
                ..
            } => formatter
                .debug_struct("Sealed")
                .field("issuance_id", issuance_id)
                .field("plan_id", plan_id)
                .field("prior", prior)
                .field("expected_current", expected_current)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for RecoveryAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (
                Self::Sealed {
                    issuance_id: left_id,
                    plan_id: left_plan,
                    prior: left_prior,
                    expected_current: left_current,
                    ..
                },
                Self::Sealed {
                    issuance_id: right_id,
                    plan_id: right_plan,
                    prior: right_prior,
                    expected_current: right_current,
                    ..
                },
            ) => {
                left_id == right_id
                    && left_plan == right_plan
                    && left_prior == right_prior
                    && left_current == right_current
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
