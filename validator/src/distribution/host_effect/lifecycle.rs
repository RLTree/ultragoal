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

pub(crate) use binding::{
    AcceptedHostEffect, AcceptedHostScope, AcceptedHostState, AcceptedLifecycleOperation,
    AcceptedLifecyclePlan, AcceptedReconciliationPolicy, AcceptedRollbackPolicy,
    HostObjectIdentity, ObservedTargetIdentity, RootPlanCustody,
};
pub(crate) use coordinator::{
    DescriptorExecutionAdapter, DescriptorExecutionCapability, DescriptorExecutionHandoff,
    DescriptorExecutionPlatform, DescriptorExecutionPrimitive, HostTargetLease, HostTargetObserver,
    RootTrustedClock, SupportedHostLifecycleCoordinator, TrustedTimeSample,
};
pub(in crate::distribution::host_effect) use recovery::{
    ExpectedPublicationObjectIdentity, PublicationAcknowledgementIdentity, PublicationExpectation,
};
pub(crate) use recovery::{
    PublicationClassification, PublicationClassificationId, PublicationInventoryObservation,
    PublicationObjectKind, PublicationObjectObservation, RecoveryProposal, RecoveryProposalAction,
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
