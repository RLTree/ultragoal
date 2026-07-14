use super::*;
use crate::distribution::host_effect::executor::model::CommandCapture;
use crate::distribution::host_effect::executor::process::BackendFailure;
use crate::distribution::host_effect::executor::target::{FaultPoint, set_fault};
use crate::distribution::host_effect::lifecycle::{
    AcceptedHostScope, DescriptorExecutionPrimitive, HostTargetObserver,
    PublicationAcknowledgementIdentity, PublicationClassificationId, SupportedHostLifecycleErrorId,
    TrustedTimeSample, lifecycle_error,
};
use crate::distribution::host_effect::{
    DurableHostEffectLedger, FileHostEffectLedger, HostEffectAuthority, HostEffectDecision,
    HostEffectLedgerError, HostEffectLedgerErrorId, HostEffectLedgerHead, HostEffectLedgerRecord,
    HostEffectPermitBinding, HostEffectReservation, HostEffectState, HostEffectTransition,
    PinnedHostExecutable,
};
use crate::distribution::{
    HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, PackageIdentity, SourceIdentity,
};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

include!("next_fixture.rs");

include!("failing_clock_sample.rs");

include!("commit_then_fail_terminal_ledger_head.rs");

include!("unavailable_observation_ledger_head.rs");

include!("unrelated_record.rs");

include!("wrong_permit_post_reservation_terminal_observation_falls_back_bound_unavailable.rs");

include!("post_reservation_correct_permit_terminal_positive_control_is_exact_and_bound.rs");

include!("positive_actual_publication_fsync_rename_ledger_and_ack_transaction_settles.rs");

include!("negative_clock_failure_terminal_transition_also_returns_exact_recovery.rs");

include!("assert_unrelated_head_advance_returns_bound_unavailable_recovery.rs");

include!("security_mutated_terminal_record_is_rejected_without_false_terminal_claim.rs");

include!("committed/recovery_after_terminal_failure.rs");

include!("committed/identity_after_observation_loss.rs");

include!(
    "dual_failure_after_committed_publication_retains_identity_when_reobservation_is_unavailable.rs"
);

include!("rename_race_and_output_overflow_never_settle.rs");

include!("executable_mutation_and_unsafe_namespace_objects_fail_before_backend_effect.rs");
