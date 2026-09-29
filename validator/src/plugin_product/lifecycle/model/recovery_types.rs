#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryToken {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) prior: LifecycleState,
    pub(crate) expected_current: LifecycleState,
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
