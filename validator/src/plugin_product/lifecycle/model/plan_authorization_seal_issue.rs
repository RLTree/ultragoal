impl PlanAuthorizationSeal {
    pub(super) fn issue(observed: &LifecycleState, request: &LifecycleRequest) -> Self {
        Self::Sealed {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            observed: observed.clone(),
            request: request.clone(),
            action_state: Arc::new(AtomicU8::new(LifecycleActionState::Planned as u8)),
            recovery_state: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn authorized_context(&self) -> Option<(&LifecycleState, &LifecycleRequest)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed {
                observed, request, ..
            } => Some((observed, request)),
        }
    }

    pub(super) fn consume(&self) -> Result<(), LifecycleError> {
        let Self::Sealed { action_state, .. } = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        action_state
            .compare_exchange(
                LifecycleActionState::Planned as u8,
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
        let Self::Sealed {
            action_state,
            recovery_state: authorized_recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedPlan);
        };
        if let Some(state) = recovery_state {
            state.validate()?;
        }
        let mut stored_recovery_state = authorized_recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        *stored_recovery_state = recovery_state.cloned();
        let next = if recovery_state.is_some() {
            LifecycleActionState::RecoveryAvailable
        } else {
            LifecycleActionState::Closed
        };
        action_state
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
    ) -> Result<(Arc<AtomicU8>, Arc<Mutex<Option<LifecycleState>>>), LifecycleError> {
        let Self::Sealed {
            action_state,
            recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedPlan);
        };
        Ok((Arc::clone(action_state), Arc::clone(recovery_state)))
    }
}

impl Default for PlanAuthorizationSeal {
    fn default() -> Self {
        Self::Unsealed
    }
}

impl fmt::Debug for PlanAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed {
                issuance_id,
                observed,
                request,
                ..
            } => formatter
                .debug_struct("Sealed")
                .field("issuance_id", issuance_id)
                .field("observed", observed)
                .field("request", request)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for PlanAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (
                Self::Sealed {
                    issuance_id: left_id,
                    observed: left_observed,
                    request: left_request,
                    ..
                },
                Self::Sealed {
                    issuance_id: right_id,
                    observed: right_observed,
                    request: right_request,
                    ..
                },
            ) => {
                left_id == right_id
                    && left_observed == right_observed
                    && left_request == right_request
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

#[derive(Clone)]
pub(super) enum RecoveryAuthorizationSeal {
    Unsealed,
    Sealed {
        issuance_id: u64,
        plan_id: String,
        prior: LifecycleState,
        expected_current: LifecycleState,
        action_state: Arc<AtomicU8>,
        recovery_state: Arc<Mutex<Option<LifecycleState>>>,
    },
}
