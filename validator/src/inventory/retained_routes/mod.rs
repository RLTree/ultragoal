mod claims;
mod commands;
mod lanes;
mod manifest;
mod model;
mod specs;
mod verify;

#[cfg(test)]
pub(crate) use model::{
    ACTIVE_READER_WRITER_STATE, AUTHORITY_CLASSIFICATION, COMPATIBILITY_BEHAVIOR,
    COMPATIBILITY_BOUNDARY, EQUIVALENCE_PROOF, INTENDED_DISPOSITION, OBSERVED_AUTHORITY_STATE,
    PHYSICAL_CLEANUP_STATE, REPLACEMENT_STATE, RESULT_PATH, RouteSpec,
};
pub(crate) use model::{
    CatalogEvidence, DigestEvidence, EntryEvidence, MatcherEvidence, RegistryRouteEvidence,
    TransitionEvidence,
};
#[cfg(test)]
pub(crate) use specs::{ROUTE_COUNT, TARGET_COUNT, by_route_id, routes, targets};
pub(crate) use specs::{by_stable_id, is_source_kind, is_target_id};
pub(crate) use verify::verify_catalog;
#[cfg(test)]
pub(crate) use verify::{VerificationError, VerifiedCatalog, registry_route_is_compiled};
