use super::{RouteRule, RoutingData};
use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::read_bounded;
use crate::inventory::retained_routes::{
    CatalogEvidence, DigestEvidence, EntryEvidence, MatcherEvidence, RegistryRouteEvidence,
    TransitionEvidence, by_stable_id, is_target_id, verify_catalog,
};
use crate::inventory::routing_state::{
    CompatibilityBehavior, CompatibilityBoundary, EquivalenceProof, ObservedAuthorityState,
    PhysicalCleanupState, ReaderWriterState, ReplacementState,
};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::BTreeSet;
use std::path::Path;

include!("max_pending_source_bytes.rs");

include!("routing_data_classify_pending_authority.rs");
