#[path = "../src/migration/mod.rs"]
mod migration;

use migration::{
    CompatibilityRoute, CompatibilityRouteDefinition, DestructiveAuthorization,
    DestructiveEffectAuthority, EvidenceVerdict, InventorySurface, InventorySurfaceObservation,
    MigrationError, MigrationInventory, MigrationPlan, MigrationPlanProjection, Od009Decision,
    ReplacementEvidence, ReplacementEvidenceAuthority, ReplacementLedgerBinding,
    RetirementDecision, RetirementReview, RetirementReviewAuthority, RetirementStatus,
    RetirementTarget, RetirementTargetProjection, SurfaceFileKind, SurfaceStatus,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

include!("migration_contract/next_root.rs");

include!("migration_contract/test_replacement_authority_current.rs");

include!("migration_contract/test_replacement_authority_authority_id.rs");

include!("migration_contract/test_replacement_authority_claim_final_reconciliation.rs");

include!("migration_contract/test_effect_authority.rs");

include!("migration_contract/unknown_or_inactive_canonical_target_is_rejected.rs");

include!(
    "migration_contract/replacement_reviewer_and_issuer_must_be_independent_of_route_owner.rs"
);

include!("migration_contract/persistent_replacement_ledger_rejects_cross_adapter_replay.rs");

include!("migration_contract/opaque_authority_debug_is_bounded_and_never_echoes_fields.rs");

include!("migration_contract/final_session_rotation_invalidates_mutate_restore_reuse.rs");

include!(
    "migration_contract/retirement_review_issuance_rejects_same_principal_session_expiry_and_stale_binding.rs"
);

include!(
    "migration_contract/destructive_authorization_issuance_rejects_shared_authority_and_ambiguous_scope.rs"
);

include!(
    "migration_contract/substituted_expired_conflicting_and_replayed_destructive_authorizations_fail_closed.rs"
);

include!("migration_contract/authority_mutation_during_one_shot_consumption_is_revalidated.rs");

include!(
    "migration_contract/external_callers_cannot_mint_clone_or_deserialize_replacement_or_retirement_authority.rs"
);

include!("migration_contract/renamed_only_or_receipt_only_replacement_is_not_proof.rs");
