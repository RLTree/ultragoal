//! Context-bound, deterministic semantic inventory.
//!
//! The builder performs read-only, path-confined discovery. Generated and handwritten
//! projections are observations only and cannot author the returned catalog.

mod builder;
mod component_expectations;
mod components;
mod digest;
mod discovery;
mod fs;
mod generated;
mod legacy;
mod projection;
#[cfg(test)]
mod race_tests;
mod registry;
mod routing;
mod schema_references;
mod types;
mod validate;
mod walk;

pub const ADOPTED_HANDOFF_DIGEST_CONFIG_KEY: &str = "ultragoal.adopted_handoff_manifest_sha256";
pub const ADOPTED_HANDOFF_MANIFEST_SHA256: &str =
    "d61c897a68d3aa985996f595a17c80f49e0730d07434b6b81de36878ef28dc51";

pub use builder::InventoryBuilder;
pub use types::{
    ActiveStatus, AuthorityCatalog, AuthorityState, FindingSeverity, GeneratedSurfaceIndex,
    InventoryEntry, InventoryError, InventoryFinding, ProjectionComparison,
};
