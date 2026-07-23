//! Context-bound, deterministic semantic inventory.
//!
//! The builder performs read-only, path-confined discovery. Generated and handwritten
//! projections are observations only and cannot author the returned catalog.

pub(crate) mod behavioral_role;
mod builder;
mod component_expectations;
mod components;
mod context_scopes;
mod digest;
mod discovery;
mod fs;
mod generated;
mod legacy;
mod migration_plan;
mod migration_registry;
#[path = "plugin/hooks.rs"]
mod plugin_hooks;
#[path = "plugin/manifest/mod.rs"]
mod plugin_manifest;
#[path = "plugin/manifest/hook/document.rs"]
mod plugin_manifest_hook_document;
#[path = "plugin/manifest/hook/matcher.rs"]
mod plugin_manifest_hook_matcher;
mod plugin_manifest_hooks;
#[path = "plugin/manifest/interface.rs"]
mod plugin_manifest_interface;
#[path = "plugin/manifest/json.rs"]
mod plugin_manifest_json;
#[path = "plugin/manifest/path.rs"]
mod plugin_manifest_path;
#[cfg(test)]
#[path = "plugin/manifest/semver.rs"]
mod plugin_manifest_semver;
mod projection;
#[cfg(test)]
mod race_tests;
mod registry;
pub(crate) use registry::inspection;
#[cfg(test)]
pub(crate) mod retained_routes;
#[cfg(not(test))]
mod retained_routes;
mod routing;
#[path = "routing/state.rs"]
mod routing_state;
mod schema_references;
mod types;
mod validate;
mod walk;

pub const ADOPTED_HANDOFF_DIGEST_CONFIG_KEY: &str = "ultragoal.adopted_handoff_manifest_sha256";
pub(crate) use migration_registry::{
    MAX_MIGRATION_REGISTRY_BYTES, MIGRATION_REGISTRY_PATH, ObservedMigrationRegistry,
};
pub const ADOPTED_HANDOFF_MANIFEST_SHA256: &str =
    "89b0d7f17aca16c262500533677e54803643939a19c71ec2fe71fb395aeb97ea";

pub use builder::InventoryBuilder;
pub(crate) use migration_plan::MigrationPlanAdapterError;
#[cfg(test)]
pub(crate) use types::AuthorityCatalogDefinition;
pub use types::{
    ActiveStatus, AuthorityCatalog, AuthorityState, FindingSeverity, GeneratedSurfaceIndex,
    InventoryEntry, InventoryError, InventoryFinding, ProjectionComparison,
};
