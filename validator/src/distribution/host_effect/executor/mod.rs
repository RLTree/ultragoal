//! Internal supported-host transaction executor candidate.
//!
//! The accepted lifecycle coordinator remains the only source of an
//! `AuthorizedHostEffect`. Darwin remains unsupported at that coordinator
//! boundary because the host has no retained-descriptor execution primitive.
//! This module does not add a public route or weaken that decision. It supplies
//! the dependency-closed transaction consumer, a descriptor-capable process
//! backend for Linux/FreeBSD, and descriptor-relative durable publication used
//! by the executor after a retained-authority handoff.

#[path = "reservation/lifecycle_completion/mod.rs"]
mod lifecycle_completion;
mod model;
mod process;
mod target;

#[cfg(not(test))]
pub(crate) use lifecycle_completion::reserve_in_flight_lifecycle;
pub(crate) use lifecycle_completion::{
    DurableHostLifecycleAdmission, HostEffectCompletion, HostEffectCompletionOutcome,
};

#[cfg(test)]
pub(crate) use model::HostEffectPostPublicationRecoveryClassification;
pub(crate) use model::{
    HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutionReceipt,
    HostEffectExecutorErrorId, HostEffectExecutorFailure,
    HostEffectPostReservationLedgerClassification,
    HostEffectPostReservationPublicationClassification, HostEffectRecoveryHandoff,
    HostEffectTerminalRecoveryClassification,
};
pub(crate) use process::NativeRetainedDescriptorProcessBackend;
use process::RetainedDescriptorProcessBackend;
pub(crate) use process::execute_bounded_observation;
pub(crate) use target::ConfinedHostEffectTarget;

use self::model::{
    CommandCaptureDigest, PostReservationRecoveryRequest, TerminalTransitionRecoveryRequest,
    digest_json,
};
use self::process::BackendFailure;
use self::target::{CommittedPublication, PublicationFailure};
use super::lifecycle::{
    DescriptorExecutionCapability, DescriptorExecutionHandoff, DescriptorExecutionPlatform,
    HostTargetLease, PublicationAcknowledgementIdentity, PublicationClassificationId,
    PublicationInventoryObservation, RootTrustedClock,
};
use super::{
    AuthorizedHostEffect, DurableHostEffectLedger, HostEffectLedgerHead, HostEffectLedgerRecord,
    HostEffectOutcome, HostEffectState, HostEffectTransition,
};
use serde::Serialize;

include!("prior_publication_evidence.rs");

include!("reservation/execution.rs");

include!("reservation/preparation.rs");

include!("reservation/command_execution.rs");

include!("publication/write.rs");

include!("publication/acknowledgement.rs");

include!("preflight.rs");

include!("recovery/backend_failure.rs");

include!("recovery/terminal_failure.rs");

include!("publication/failure.rs");

include!("recovery/preterminal_failure.rs");

include!("recovery/post_reservation_reobservation.rs");

include!("publication_bytes.rs");

#[cfg(test)]
mod tests;
