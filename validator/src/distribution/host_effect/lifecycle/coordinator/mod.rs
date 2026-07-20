use super::super::{
    AuthorizedHostEffect, DurableHostEffectLedger, HostEffectAuthority, PinnedHostExecutable,
};
#[cfg(test)]
use super::super::{HostEffectState, HostEffectTransition};
use super::binding::{AcceptedHostEffect, HostEffectAcceptanceRequest, ObservedTargetIdentity};
use super::recovery::{
    issue_recovery_authorization, propose_recovery, PublicationClassification,
    RecoveryAuthorization, RecoveryProposal,
};
use super::{lifecycle_error, SupportedHostLifecycleError, SupportedHostLifecycleErrorId};
#[cfg(test)]
use crate::plugin_product::lifecycle::HostLifecycleCustody;
#[cfg(not(test))]
use crate::plugin_product::lifecycle::HostLifecycleCustody;
use serde::Serialize;
use sha2::{Digest, Sha256};

include!("authority_nonce_bytes.rs");

include!("host_target_observer.rs");

include!("supported/binding.rs");

include!("supported/recovery_proposal.rs");
