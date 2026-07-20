use super::archive;
use super::manifest;
use super::materialize::ExpectedTree;
use super::output::{
    PackageArtifactBinding, PackageArtifactTransaction, publish_package_artifact,
    rollback_package_artifact,
};
use super::plan::{PackageEntry, PackagePlan, entry_tree_sha256};
use super::snapshot::{PackageSnapshot, verify_package};
use super::spec::PackageRole;
use crate::context::LiveContext;
use crate::distribution::filesystem::{ScopedFile, ScopedTree};
use crate::distribution::host_capability::JourneyBinding;
use crate::distribution::reader::sha256;
use crate::inventory::AuthorityCatalog;
use crate::package::inventory::snapshot::{
    PackageCapture, PackageSnapshot as SourcePackageSnapshot,
};
use crate::plugin_manifest;
use serde::Serialize;

include!("plugin_id.rs");

include!("verify_artifact_against_source.rs");

include!("packaged_entries.rs");

include!("inventory_publication.rs");

include!("archive_publication.rs");

#[cfg(test)]
mod tests;
