mod claims;
mod commands;
mod lanes;
mod manifest;
mod model;
mod specs;
mod verify;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub(crate) use model::{
    CatalogEvidence, DigestEvidence, EntryEvidence, MatcherEvidence, RegistryRouteEvidence,
    TransitionEvidence,
};
#[cfg(test)]
pub(crate) use specs::{ROUTE_COUNT, routes};
pub(crate) use specs::{by_stable_id, is_target_id};
pub(crate) use verify::verify_catalog;
