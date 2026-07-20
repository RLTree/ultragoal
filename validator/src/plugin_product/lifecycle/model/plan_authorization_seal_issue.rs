impl PlanAuthorizationSeal {
    pub(super) fn issue(observed: &LifecycleState, request: &LifecycleRequest) -> Self {
        Self::Sealed(Box::new(PlanAuthorizationSealData {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            observed: observed.clone(),
            request: request.clone(),
            action_state: Arc::new(AtomicU8::new(LifecycleActionState::Planned as u8)),
            recovery_state: Arc::new(Mutex::new(None)),
        }))
    }

    pub(super) fn authorized_context(&self) -> Option<(&LifecycleState, &LifecycleRequest)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed(data) => Some((&data.observed, &data.request)),
        }
    }

    pub(super) fn consume(&self) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        data.action_state
            .compare_exchange(
                LifecycleActionState::Planned as u8,
                LifecycleActionState::Applying as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::ReplayedPlan)
    }

    pub(super) fn transfer_to_host(&self) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        data.action_state
            .compare_exchange(
                LifecycleActionState::Planned as u8,
                LifecycleActionState::Transferred as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::ReplayedPlan)
    }

    pub(super) fn consume_transferred(&self) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        data.action_state
            .compare_exchange(
                LifecycleActionState::Transferred as u8,
                LifecycleActionState::Applying as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::ReplayedPlan)
    }

    pub(super) fn finish_apply(
        &self,
        recovery_state: Option<&LifecycleState>,
    ) -> Result<(), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        if let Some(state) = recovery_state {
            state.validate()?;
        }
        let mut stored_recovery_state = data
            .recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        *stored_recovery_state = recovery_state.cloned();
        let next = if recovery_state.is_some() {
            LifecycleActionState::RecoveryAvailable
        } else {
            LifecycleActionState::Closed
        };
        data.action_state
            .compare_exchange(
                LifecycleActionState::Applying as u8,
                next as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::InvalidTransition)
    }

    pub(super) fn recovery_authority(
        &self,
    ) -> Result<(LifecycleActionAuthority, RecoveryStateAuthority), LifecycleError> {
        let Self::Sealed(data) = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        Ok((
            Arc::clone(&data.action_state),
            Arc::clone(&data.recovery_state),
        ))
    }
}

impl fmt::Debug for PlanAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed(data) => formatter
                .debug_struct("Sealed")
                .field("issuance_id", &data.issuance_id)
                .field("observed", &data.observed)
                .field("request", &data.request)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for PlanAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (Self::Sealed(left), Self::Sealed(right)) => {
                left.issuance_id == right.issuance_id
                    && left.observed == right.observed
                    && left.request == right.request
            }
            _ => false,
        }
    }
}

impl Eq for PlanAuthorizationSeal {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyDisposition {
    Applied,
    RecoveredAfterFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReport {
    pub plan_id: String,
    pub disposition: ApplyDisposition,
    pub state: LifecycleState,
    pub completed_effects: Vec<LifecycleEffect>,
    pub failed_effect: Option<LifecycleEffect>,
    pub causal_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryToken {
    pub schema_version: String,
    pub plan_id: String,
    pub prior: LifecycleState,
    pub expected_current: LifecycleState,
    #[serde(skip, default)]
    pub(super) authorization_seal: RecoveryAuthorizationSeal,
}

#[derive(Clone, Default)]
pub(super) enum RecoveryAuthorizationSeal {
    #[default]
    Unsealed,
    Sealed(Box<RecoveryAuthorizationSealData>),
}

#[derive(Clone)]
pub(super) struct RecoveryAuthorizationSealData {
    issuance_id: u64,
    plan_id: String,
    prior: LifecycleState,
    expected_current: LifecycleState,
    action_state: LifecycleActionAuthority,
    recovery_state: RecoveryStateAuthority,
}
