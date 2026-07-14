use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::product_adapter::catalog::CANONICAL_TEMPLATES;
use crate::repository_fit::product_adapter::protocol::OpaqueFitApplyRequest;
use crate::repository_fit::product_adapter::root_permit::{
    ProtectedCaptureBoundary, ProtectedCapturePhase, ReconciliationTargetPhase,
    RepositoryFitApplyFailure, RepositoryFitApplyOutcome, RepositoryFitPermitEffects,
    TargetCapturePhase, TestRepositoryFitPermitAuthority, apply_with_root_permit,
    assert_protected_capture_hook_consumed_for_test,
    assert_reconciliation_target_hook_consumed_for_test,
    assert_target_capture_hook_consumed_for_test, before_final_green_observation_for_test,
    before_postflight_observation_for_test, duplicate_authorization_for_test,
    permit_seal_stage_for_test, protected_capture_hook_for_test,
    reconciliation_target_hook_for_test, scope_violation_for_test, target_capture_hook_for_test,
};
use crate::repository_fit::product_adapter::{
    AdapterErrorId, plan_target, prepare_apply_request, verify_target,
};
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader, digest,
};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread::JoinHandle;

use super::scenario::{git_status, snapshot};

#[path = "arm_alternating_target_hybrid.rs"]
mod arm_alternating_target_hybrid;
#[path = "assert_final_green_ancestor_drift.rs"]
mod assert_final_green_ancestor_drift;
#[path = "authority_rejects_short_secrets_nonces_and_invalid_validity_windows.rs"]
mod authority_rejects_short_secrets_nonces_and_invalid_validity_windows;
#[path = "complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes.rs"]
mod complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes;
#[path = "descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects.rs"]
mod descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects;
#[path = "final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba.rs"]
mod final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba;
#[path = "missing_permit_missing_lease_and_time_windows_refuse_before_effect_without_consumption.rs"]
mod missing_permit_missing_lease_and_time_windows_refuse_before_effect_without_consumption;
#[path = "protected_change_version_rejects_same_inode_mutate_restore_aba.rs"]
mod protected_change_version_rejects_same_inode_mutate_restore_aba;
#[path = "protected_descendants_inside_managed_ancestors_are_preserved_and_bound.rs"]
mod protected_descendants_inside_managed_ancestors_are_preserved_and_bound;
#[path = "protected_descriptor_capture_rejects_rollback_ab_swap.rs"]
mod protected_descriptor_capture_rejects_rollback_ab_swap;
#[path = "reconciliation_binds_every_target_collect_to_the_authorized_snapshot.rs"]
mod reconciliation_binds_every_target_collect_to_the_authorized_snapshot;
#[path = "scenario_fixture.rs"]
mod scenario_fixture;
#[path = "source_manifest_authority_observed_modes_and_desired_digest_are_individually_bound.rs"]
mod source_manifest_authority_observed_modes_and_desired_digest_are_individually_bound;
#[path = "structurally_equal_cross_session_request_cannot_consume_another_request_permit.rs"]
mod structurally_equal_cross_session_request_cannot_consume_another_request_permit;
#[path = "target_complete_double_collect_rejects_two_leaf_alternating_hybrid.rs"]
mod target_complete_double_collect_rejects_two_leaf_alternating_hybrid;
#[path = "undeclared_write.rs"]
mod undeclared_write;
#[path = "within_postflight_and_final_capture_swaps_are_terminally_ambiguous.rs"]
mod within_postflight_and_final_capture_swaps_are_terminally_ambiguous;

pub(crate) use arm_alternating_target_hybrid::*;
pub(crate) use assert_final_green_ancestor_drift::*;
pub(crate) use authority_rejects_short_secrets_nonces_and_invalid_validity_windows::*;
pub(crate) use complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes::*;
pub(crate) use descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects::*;
pub(crate) use final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba::*;
pub(crate) use missing_permit_missing_lease_and_time_windows_refuse_before_effect_without_consumption::*;
pub(crate) use protected_change_version_rejects_same_inode_mutate_restore_aba::*;
pub(crate) use protected_descendants_inside_managed_ancestors_are_preserved_and_bound::*;
pub(crate) use protected_descriptor_capture_rejects_rollback_ab_swap::*;
pub(crate) use reconciliation_binds_every_target_collect_to_the_authorized_snapshot::*;
pub(crate) use scenario_fixture::*;
pub(crate) use source_manifest_authority_observed_modes_and_desired_digest_are_individually_bound::*;
pub(crate) use structurally_equal_cross_session_request_cannot_consume_another_request_permit::*;
pub(crate) use target_complete_double_collect_rejects_two_leaf_alternating_hybrid::*;
pub(crate) use undeclared_write::*;
pub(crate) use within_postflight_and_final_capture_swaps_are_terminally_ambiguous::*;
