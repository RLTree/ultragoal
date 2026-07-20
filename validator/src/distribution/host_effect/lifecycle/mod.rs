//! Root-coordinated supported-host lifecycle boundary.
//!
//! This module is crate-private. The parent module wires an internal transaction
//! executor, but a retained-descriptor platform adapter must still be selected
//! before authority issuance or durable-ledger reservation; unsupported
//! platforms fail at that boundary.

use serde::Serialize;

mod binding;
mod coordinator;
mod recovery;

#[cfg(test)]
pub(in crate::distribution::host_effect) use binding::HostEffectAcceptanceRequest;
#[cfg(test)]
pub(crate) use binding::{
    AcceptedHostEffect, AcceptedHostState, AcceptedLifecycleOperation, AcceptedLifecyclePlan,
    AcceptedReconciliationPolicy, AcceptedRollbackPolicy,
};
pub(crate) use binding::{AcceptedHostScope, HostObjectIdentity, ObservedTargetIdentity};
#[cfg(test)]
pub(in crate::distribution::host_effect) use coordinator::HostEffectPreparationRequest;
#[cfg(test)]
pub(crate) use coordinator::TrustedTimeSample;
#[cfg(test)]
pub(crate) use coordinator::{
    DescriptorExecutionAdapter, DescriptorExecutionPrimitive, SupportedHostLifecycleCoordinator,
};
pub(crate) use coordinator::{
    DescriptorExecutionCapability, DescriptorExecutionHandoff, DescriptorExecutionPlatform,
    HostTargetLease, HostTargetObserver, RootTrustedClock,
};
#[cfg(test)]
pub(crate) use recovery::RecoveryProposalAction;
pub(in crate::distribution::host_effect) use recovery::{
    ExpectedPublicationObjectIdentity, ExpectedRegularPublicationObject,
    PublicationAcknowledgementIdentity, PublicationExpectation,
    PublicationInventoryObservationRequest, PublicationObjectObservationRequest,
};
pub(crate) use recovery::{
    PublicationClassification, PublicationClassificationId, PublicationInventoryObservation,
    PublicationObjectKind, PublicationObjectObservation,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SupportedHostLifecycleErrorId {
    UnsupportedPlatform,
    DescriptorExecutionUnavailable,
    InvalidAcceptedIdentity,
    CoordinatorSubstitution,
    PlanSubstitution,
    ExecutableSubstitution,
    TargetSubstitution,
    TargetRace,
    UntrustedTime,
    StaleLedgerHead,
    AuthorityRejected,
    LedgerRejected,
    HandoffConstructionFailed,
    RecoveryAuthorizationRequired,
    RecoveryUnsafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SupportedHostLifecycleError {
    id: SupportedHostLifecycleErrorId,
}

impl SupportedHostLifecycleError {
    pub(crate) const fn id(&self) -> SupportedHostLifecycleErrorId {
        self.id
    }
}

pub(super) const fn lifecycle_error(
    id: SupportedHostLifecycleErrorId,
) -> SupportedHostLifecycleError {
    SupportedHostLifecycleError { id }
}

#[cfg(test)]
mod tests;
