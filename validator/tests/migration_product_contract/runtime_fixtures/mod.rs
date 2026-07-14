use crate::migration::product::{
    AdoptedRegistrySnapshot, ApplyAuthorizationAuthority, AuthoritySnapshot,
    ConfinedMigrationEffect, DurableMigrationStore, EffectFault, EffectObservation, JournalPhase,
    MigrationInputBinding, MigrationInputSource, MigrationOperation, PlanDisposition,
    PlannedMigrationEffect, ProductInputSnapshot, ProductMigrationError, ProductMigrationPlan,
    ReservationRequest, ReservationResult, StoreFault, derive_product_plan,
};
use crate::migration::{
    InventorySurface, InventorySurfaceObservation, MigrationInventory, SurfaceFileKind,
    SurfaceStatus,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

include!("sha_hash.rs");

include!("input_with_routes.rs");

include!("fake/authority_boundary.rs");

include!("fake/store_fail_on_cas.rs");

include!("fake/effects_for_plan.rs");
