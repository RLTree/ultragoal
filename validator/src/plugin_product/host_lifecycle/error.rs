use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HostLifecycleErrorId {
    InvalidBinding,
    LifecycleRejected,
    ObservationUnavailable,
    ObservationChanged,
    ObservationConflict,
    ObservationTransactionUnsupported,
    UnsupportedSubstitution,
    IdentityMismatch,
    ExternalEffectNotEligible,
    ExternalEffectSessionMismatch,
    ExternalEffectReplayed,
    HostCapabilityRejected,
    HostScopeRejected,
    SessionStateRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostLifecycleError {
    id: HostLifecycleErrorId,
}

impl HostLifecycleError {
    pub(crate) const fn new(id: HostLifecycleErrorId) -> Self {
        Self { id }
    }

    pub const fn id(self) -> HostLifecycleErrorId {
        self.id
    }
}

impl fmt::Display for HostLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.id {
            HostLifecycleErrorId::InvalidBinding => "host lifecycle binding is invalid",
            HostLifecycleErrorId::LifecycleRejected => "host lifecycle transition was rejected",
            HostLifecycleErrorId::ObservationUnavailable => {
                "required host observation is unavailable"
            }
            HostLifecycleErrorId::ObservationChanged => "host observation changed during capture",
            HostLifecycleErrorId::ObservationConflict => "host observation conflicts with binding",
            HostLifecycleErrorId::ObservationTransactionUnsupported => {
                "transactional host observation is unsupported"
            }
            HostLifecycleErrorId::UnsupportedSubstitution => {
                "unsupported host surface cannot accept supplied evidence"
            }
            HostLifecycleErrorId::IdentityMismatch => "host identity chain does not match",
            HostLifecycleErrorId::ExternalEffectNotEligible => {
                "external host effect is not eligible in this session state"
            }
            HostLifecycleErrorId::ExternalEffectSessionMismatch => {
                "external host effect belongs to a different session issuance"
            }
            HostLifecycleErrorId::ExternalEffectReplayed => {
                "external host effect eligibility was already consumed"
            }
            HostLifecycleErrorId::HostCapabilityRejected => {
                "required host capability is not supported"
            }
            HostLifecycleErrorId::HostScopeRejected => {
                "external host effect scope does not match the host declaration"
            }
            HostLifecycleErrorId::SessionStateRejected => {
                "host lifecycle session state rejected the operation"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HostLifecycleError {}
