use crate::distribution::{
    DistributionErrorId as ErrorId, ExpectedTree, MaterializeEffects, PackageEffects, TreeObject,
    TreeObjectKind, build_package, materialize_package, plan_package_from_inventory,
    rollback_materialization, tree_sha256,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture, digest, tree};
use crate::package_manifest::manifest;
use serde::Serialize;
use std::fs;

include!("package_identity/catalog.rs");

include!("package_identity/accepted_sources_and_destination_reject_aliases_and_special_objects.rs");
