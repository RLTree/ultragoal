use super::super::{
    DurableHostEffectLedger, FileHostEffectLedger, HostEffectAuthority, HostEffectLedgerError,
    HostEffectLedgerErrorId, HostEffectLedgerHead, HostEffectLedgerRecord, HostEffectReservation,
    HostEffectState, HostEffectTransition, SelectedCodexExecutable,
    SelectedCodexExecutableTestFixture, selected_test_fixture,
};
use super::binding::AcceptedHostEffect;
use super::recovery::{
    ExpectedPublicationObjectIdentity, PublicationAcknowledgementIdentity, PublicationExpectation,
};
use super::*;
use crate::distribution::{
    HostCapabilityDeclaration, HostCommandPlan, HostCommandPlanProjection, JourneyBinding,
    PackageIdentity, SourceIdentity,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

include!("next_fixture.rs");
include!("recording_ledger.rs");

include!("recording_ledger_head.rs");

include!("probe_target_lease_identity.rs");

include!("wrong/rejected_primitive_adapters.rs");

include!("wrong/pre_reservation_identity.rs");

include!("wrong/package_plan.rs");

include!("accepted_lifecycle_operation_matrix.rs");

include!("every_complete_permit_binding_dimension_changes_the_canonical_binding.rs");

include!("expected/publication_identity.rs");

include!("acknowledgements_reject_effect_publication_head_and_wire_substitution.rs");

include!("special_multiple_and_generation_races_fail_before_recovery_acceptance.rs");

include!("recovery_authorization_rejects_generation_only_ledger_head_substitution.rs");

include!("expected/regular_file.rs");
