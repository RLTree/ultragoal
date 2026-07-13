use super::archive;
use super::manifest;
use super::materialize::{ExpectedTree, MaterializeEffects};
use super::output::{
    PackageArtifactBinding, PackageArtifactTransaction, publish_package_artifact,
    rollback_package_artifact,
};
use super::plan::{PackageEntry, PackagePlan, entry_tree_sha256};
use super::snapshot::{PackageSnapshot, verify_package};
use super::spec::PackageRole;
use crate::context::LiveContext;
use crate::distribution::reader::sha256;
use crate::inventory::AuthorityCatalog;
use crate::package::inventory::snapshot::{
    PackageCapture, PackageSnapshot as SourcePackageSnapshot,
};
use crate::plugin_manifest;
use serde::Serialize;

pub(super) const PLUGIN_ID: &str = "harness-ultragoal";
pub(super) const SUPPORTED_VERSION: &str = "0.0.12";
pub(super) const SUPPORTED_MANIFEST_PATH: &str = ".codex-plugin/plugin.json";
pub(super) const CANONICAL_SKILLS: [&str; 8] = [
    "harness-ultragoal",
    "repository-fit",
    "routine-work",
    "diagnose-and-observe",
    "goal-run",
    "product-journey-review",
    "prove",
    "improve-and-maintain",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductionPackageErrorId {
    ContextUnavailable,
    CatalogMismatch,
    SourceUnavailable,
    ManifestMismatch,
    MembershipMismatch,
    ArchiveMismatch,
    InventoryMismatch,
    OutputFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionPackageError {
    id: ProductionPackageErrorId,
}

impl ProductionPackageError {
    pub(crate) const fn id(self) -> ProductionPackageErrorId {
        self.id
    }
}

impl std::fmt::Display for ProductionPackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.id {
            ProductionPackageErrorId::ContextUnavailable => {
                "supported package context is unavailable"
            }
            ProductionPackageErrorId::CatalogMismatch => {
                "supported package authority catalog does not match"
            }
            ProductionPackageErrorId::SourceUnavailable => {
                "supported package source is unavailable"
            }
            ProductionPackageErrorId::ManifestMismatch => {
                "supported package manifests do not match"
            }
            ProductionPackageErrorId::MembershipMismatch => {
                "supported package membership does not match"
            }
            ProductionPackageErrorId::ArchiveMismatch => "supported package archive does not match",
            ProductionPackageErrorId::InventoryMismatch => {
                "supported package inventory does not match"
            }
            ProductionPackageErrorId::OutputFailed => {
                "supported package output could not be published"
            }
        })
    }
}

impl std::error::Error for ProductionPackageError {}

fn failure(id: ProductionPackageErrorId) -> ProductionPackageError {
    ProductionPackageError { id }
}

pub(crate) struct ProductionPackageSession {
    context: LiveContext,
    catalog_id: String,
    capture: PackageCapture,
}

impl ProductionPackageSession {
    pub(crate) fn begin(
        context: &LiveContext,
        catalog: &AuthorityCatalog,
    ) -> Result<Self, ProductionPackageError> {
        validate_context_catalog(context, catalog)?;
        let capture = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        Ok(Self {
            context: context.clone(),
            catalog_id: catalog.catalog_id().to_owned(),
            capture,
        })
    }

    pub(crate) fn finish(self) -> Result<ProductionPackageArtifact, ProductionPackageError> {
        let source = self
            .capture
            .finish()
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        self.context
            .revalidate()
            .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
        let artifact = build_artifact(&self.context, &self.catalog_id, source.as_ref())?;
        self.context
            .revalidate()
            .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
        Ok(artifact)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProductionPackageArtifact {
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    source_snapshot_id: String,
    source_inventory: Vec<u8>,
    plan: PackagePlan,
    snapshot: PackageSnapshot,
    binding: PackageArtifactBinding,
}

impl ProductionPackageArtifact {
    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }

    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn source_snapshot_id(&self) -> &str {
        &self.source_snapshot_id
    }

    pub(crate) fn source_inventory(&self) -> &[u8] {
        &self.source_inventory
    }

    pub(crate) fn snapshot(&self) -> &PackageSnapshot {
        &self.snapshot
    }

    pub(crate) fn publish(
        &self,
        context: &LiveContext,
        catalog: &AuthorityCatalog,
        expected: &ExpectedTree,
        effects: &mut impl MaterializeEffects,
    ) -> Result<PackageArtifactTransaction, ProductionPackageError> {
        verify_product_package(self, context, catalog)?;
        let guard = PackageCapture::begin(context)
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
        let transaction =
            publish_package_artifact(&self.snapshot, &self.binding, expected, effects)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
        let post_effect = guard
            .finish()
            .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))
            .and_then(|_| verify_product_package(self, context, catalog));
        if let Err(problem) = post_effect {
            rollback_package_artifact(transaction, effects)
                .map_err(|_| failure(ProductionPackageErrorId::OutputFailed))?;
            return Err(problem);
        }
        Ok(transaction)
    }
}

pub(crate) fn capture_product_package(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<ProductionPackageArtifact, ProductionPackageError> {
    ProductionPackageSession::begin(context, catalog)?.finish()
}

pub(crate) fn verify_product_package(
    artifact: &ProductionPackageArtifact,
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<(), ProductionPackageError> {
    validate_context_catalog(context, catalog)?;
    let source = PackageCapture::begin(context)
        .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?
        .finish()
        .map_err(|_| failure(ProductionPackageErrorId::SourceUnavailable))?;
    verify_artifact_against_source(artifact, context, catalog, source.as_ref())
}

fn verify_artifact_against_source(
    artifact: &ProductionPackageArtifact,
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    source: &SourcePackageSnapshot,
) -> Result<(), ProductionPackageError> {
    validate_context_catalog(context, catalog)?;
    let current_candidate = candidate_id(context)?;
    if artifact.context_id != context.context_id()
        || artifact.candidate_id != current_candidate
        || artifact.catalog_id != catalog.catalog_id()
    {
        return Err(failure(ProductionPackageErrorId::CatalogMismatch));
    }

    let decoded = archive::decode(artifact.snapshot.archive())
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    manifest::validate(&decoded.entries, &decoded.plugin_id, &decoded.version)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let (inventory, inventory_sha256) = archive::inventory(&decoded)
        .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    if inventory != artifact.snapshot.inventory()
        || inventory_sha256 != artifact.snapshot.inventory_sha256()
        || entry_tree_sha256(&decoded.entries)
            .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?
            != decoded.source_tree_sha256
    {
        return Err(failure(ProductionPackageErrorId::InventoryMismatch));
    }

    let fresh = build_artifact(context, catalog.catalog_id(), source)?;
    if !same_artifact(artifact, &fresh) {
        return Err(failure(ProductionPackageErrorId::SourceUnavailable));
    }
    validate_context_catalog(context, catalog)
}

fn same_artifact(expected: &ProductionPackageArtifact, actual: &ProductionPackageArtifact) -> bool {
    expected.context_id == actual.context_id
        && expected.candidate_id == actual.candidate_id
        && expected.catalog_id == actual.catalog_id
        && expected.source_snapshot_id == actual.source_snapshot_id
        && expected.source_inventory == actual.source_inventory
        && expected.plan.context_id == actual.plan.context_id
        && expected.plan.candidate_id == actual.plan.candidate_id
        && expected.plan.plugin_id == actual.plan.plugin_id
        && expected.plan.version == actual.plan.version
        && expected.plan.source_date_epoch == actual.plan.source_date_epoch
        && expected.plan.catalog_id == actual.plan.catalog_id
        && expected.plan.accepted_inventory_sha256 == actual.plan.accepted_inventory_sha256
        && expected.plan.source_tree_sha256 == actual.plan.source_tree_sha256
        && expected.plan.entries == actual.plan.entries
        && expected.snapshot == actual.snapshot
        && expected.binding == actual.binding
}

fn validate_context_catalog(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<(), ProductionPackageError> {
    context
        .revalidate()
        .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
    if catalog.context_id() != context.context_id() || catalog.revalidate_identity().is_err() {
        return Err(failure(ProductionPackageErrorId::CatalogMismatch));
    }
    Ok(())
}

fn build_artifact(
    context: &LiveContext,
    catalog_id: &str,
    source: &SourcePackageSnapshot,
) -> Result<ProductionPackageArtifact, ProductionPackageError> {
    if source.context_id() != context.context_id() {
        return Err(failure(ProductionPackageErrorId::ContextUnavailable));
    }
    let version = validate_manifests(&source)?;
    let entries = packaged_entries(&source)?;
    manifest::validate(&entries, PLUGIN_ID, &version)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let candidate_id = candidate_id(context)?;
    let source_tree_sha256 = entry_tree_sha256(&entries)
        .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    let source_inventory = source_inventory(
        context.context_id(),
        &candidate_id,
        catalog_id,
        &version,
        &entries,
    )?;
    let plan = PackagePlan {
        context_id: context.context_id().to_owned(),
        candidate_id: candidate_id.clone(),
        plugin_id: PLUGIN_ID.to_owned(),
        version,
        source_date_epoch: 0,
        catalog_id: catalog_id.to_owned(),
        accepted_inventory_sha256: sha256(&source_inventory),
        source_tree_sha256,
        entries,
    };
    let archive =
        archive::encode(&plan).map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    let snapshot = verify_package(&plan, &archive)
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    let binding = PackageArtifactBinding::issue(&snapshot)
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    Ok(ProductionPackageArtifact {
        context_id: context.context_id().to_owned(),
        candidate_id,
        catalog_id: catalog_id.to_owned(),
        source_snapshot_id: source.snapshot_id().to_owned(),
        source_inventory,
        plan,
        snapshot,
        binding,
    })
}

fn validate_manifests(source: &SourcePackageSnapshot) -> Result<String, ProductionPackageError> {
    let supported_bytes = source
        .bytes(SUPPORTED_MANIFEST_PATH)
        .ok_or_else(|| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let supported = plugin_manifest::parse(supported_bytes, plugin_manifest::MANIFEST_LIMIT)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    if !plugin_manifest::semantic_issues(&supported).is_empty()
        || supported.name != PLUGIN_ID
        || supported.version != SUPPORTED_VERSION
        || supported.skills.as_deref() != Some("./skills/")
    {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    let draft = source.manifest();
    if draft.get("name").and_then(serde_json::Value::as_str) != Some(PLUGIN_ID)
        || draft.get("version").and_then(serde_json::Value::as_str) != Some(SUPPORTED_VERSION)
    {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    let skills = draft
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| failure(ProductionPackageErrorId::ManifestMismatch))?;
    if skills.len() != CANONICAL_SKILLS.len()
        || skills.iter().zip(CANONICAL_SKILLS).any(|(row, name)| {
            let expected_path = format!("skills/{name}/SKILL.md");
            row.get("name").and_then(serde_json::Value::as_str) != Some(name)
                || row.get("path").and_then(serde_json::Value::as_str)
                    != Some(expected_path.as_str())
        })
    {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    Ok(SUPPORTED_VERSION.to_owned())
}

fn packaged_entries(
    source: &SourcePackageSnapshot,
) -> Result<Vec<PackageEntry>, ProductionPackageError> {
    let mut entries = Vec::with_capacity(source.packaged_paths().len());
    for path in source.packaged_paths() {
        let bytes = source
            .bytes(path)
            .ok_or_else(|| failure(ProductionPackageErrorId::MembershipMismatch))?;
        let mode = source
            .unix_mode(path)
            .ok_or_else(|| failure(ProductionPackageErrorId::MembershipMismatch))?;
        let role = package_role(path, mode)?;
        entries.push(PackageEntry {
            path: path.clone(),
            mode,
            role,
            sha256: sha256(bytes),
            bytes: bytes.to_vec(),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if entries.len() != source.packaged_paths().len()
        || entries.windows(2).any(|pair| pair[0].path == pair[1].path)
        || entries
            .iter()
            .filter(|entry| entry.role == PackageRole::Manifest)
            .count()
            != 1
    {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    }
    for name in CANONICAL_SKILLS {
        for required in [
            format!("skills/{name}/SKILL.md"),
            format!("skills/{name}/agents/openai.yaml"),
        ] {
            if !entries.iter().any(|entry| entry.path == required) {
                return Err(failure(ProductionPackageErrorId::MembershipMismatch));
            }
        }
    }
    Ok(entries)
}

fn package_role(path: &str, mode: u32) -> Result<PackageRole, ProductionPackageError> {
    let role = if path == SUPPORTED_MANIFEST_PATH {
        PackageRole::Manifest
    } else if CANONICAL_SKILLS
        .iter()
        .any(|name| path == format!("skills/{name}/SKILL.md"))
    {
        PackageRole::Skill
    } else if CANONICAL_SKILLS
        .iter()
        .any(|name| path == format!("skills/{name}/agents/openai.yaml"))
    {
        PackageRole::Agent
    } else {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    };
    if mode != 0o644 {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    }
    Ok(role)
}

#[derive(Serialize)]
struct SourceInventory<'a> {
    schema: &'static str,
    context_id: &'a str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    plugin_id: &'static str,
    version: &'a str,
    source_date_epoch: u64,
    entries: Vec<SourceInventoryEntry<'a>>,
}

#[derive(Serialize)]
struct SourceInventoryEntry<'a> {
    path: &'a str,
    object_type: &'static str,
    mode: u32,
    sha256: &'a str,
    byte_length: u64,
    role: PackageRole,
}

fn source_inventory(
    context_id: &str,
    candidate_id: &str,
    catalog_id: &str,
    version: &str,
    entries: &[PackageEntry],
) -> Result<Vec<u8>, ProductionPackageError> {
    let rows = entries
        .iter()
        .map(|entry| SourceInventoryEntry {
            path: &entry.path,
            object_type: "regular-file",
            mode: entry.mode,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
            role: entry.role,
        })
        .collect();
    serde_json::to_vec(&SourceInventory {
        schema: "harness-ultragoal.accepted-package-source-set.v1",
        context_id,
        candidate_id,
        catalog_id,
        plugin_id: PLUGIN_ID,
        version,
        source_date_epoch: 0,
        entries: rows,
    })
    .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))
}

fn candidate_id(context: &LiveContext) -> Result<String, ProductionPackageError> {
    serde_json::to_vec(context.candidate())
        .map(|bytes| sha256(&bytes))
        .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))
}

#[cfg(test)]
mod tests;
