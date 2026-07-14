use super::super::{DarwinHostError, DarwinHostErrorId};
use super::digest;
use crate::distribution::PackageSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct PackageBinding {
    pub(in super::super) context_id: String,
    pub(in super::super) candidate_id: String,
    pub(in super::super) catalog_id: String,
    pub(in super::super) accepted_inventory_sha256: String,
    pub(in super::super) inventory_sha256: String,
    pub(in super::super) plugin_id: String,
    pub(in super::super) version: String,
    pub(in super::super) tree_sha256: String,
    pub(in super::super) archive_sha256: String,
    pub(in super::super) package_sha256: String,
}

impl PackageBinding {
    pub(in super::super) fn from_snapshot(
        snapshot: &PackageSnapshot,
    ) -> Result<Self, DarwinHostError> {
        let identity = snapshot.identity();
        let source = identity.source();
        let archive_sha256 = digest(snapshot.archive());
        if snapshot.context_id() != source.context_id()
            || snapshot.candidate_id() != source.candidate_id()
            || snapshot.catalog_id() != source.catalog_id()
            || snapshot.accepted_inventory_sha256() != source.accepted_inventory_sha256()
            || snapshot.source_tree_sha256() != identity.tree_sha256()
            || snapshot.package_sha256() != identity.archive_sha256()
            || archive_sha256 != snapshot.package_sha256()
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::InvalidPackage));
        }
        Ok(Self {
            context_id: source.context_id().to_owned(),
            candidate_id: source.candidate_id().to_owned(),
            catalog_id: source.catalog_id().to_owned(),
            accepted_inventory_sha256: source.accepted_inventory_sha256().to_owned(),
            inventory_sha256: snapshot.inventory_sha256().to_owned(),
            plugin_id: source.plugin_id().to_owned(),
            version: source.version().to_owned(),
            tree_sha256: identity.tree_sha256().to_owned(),
            archive_sha256: identity.archive_sha256().to_owned(),
            package_sha256: snapshot.package_sha256().to_owned(),
        })
    }
}
