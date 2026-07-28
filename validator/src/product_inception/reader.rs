use super::InceptionError;
use super::model::{CandidateBinding, ContractFacts};
use crate::context::{LiveContext, ReadSession, inception_subject_identity};
use crate::digest;
use crate::inventory::{AuthorityCatalog, InventoryBuilder};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(crate) const BRIEF_PATH: &str = "PRODUCT_SUCCESS_BRIEF.json";
const PRODUCT_CONTRACT_PATH: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";
const CLAIM_REGISTRY_PATH: &str =
    "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json";
const SURFACE_CATALOG_PATH: &str =
    "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json";
const GENERATED_AUTHORITY_PATH: &str = "migration/generated-surface-authority.json";
const PRODUCT_SUCCESS_CONTRACT_VERSION: &str = "2.3.0";
const MAX_BRIEF_BYTES: u64 = 1024 * 1024;
const MAX_CONTRACT_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) struct ReadResult {
    pub(crate) catalog: AuthorityCatalog,
    pub(crate) facts: ContractFacts,
    pub(crate) candidate: CandidateBinding,
    pub(crate) brief: Option<Vec<u8>>,
}
pub(crate) fn read(context: &LiveContext) -> Result<ReadResult, InceptionError> {
    context.revalidate().map_err(InceptionError::context)?;
    let catalog = InventoryBuilder::new(context)
        .build()
        .map_err(|_| InceptionError::CatalogUnavailable)?;
    read_with_catalog(context, catalog)
}
pub(crate) fn read_with_catalog(
    context: &LiveContext,
    catalog: AuthorityCatalog,
) -> Result<ReadResult, InceptionError> {
    let reads = context
        .begin_read_session()
        .map_err(InceptionError::context)?;
    let root = reads.root().to_path_buf();
    let contract = read_required(&reads, &root, PRODUCT_CONTRACT_PATH, MAX_CONTRACT_BYTES)?;
    let generated = read_required(&reads, &root, GENERATED_AUTHORITY_PATH, MAX_CONTRACT_BYTES)?;
    let claims = read_required(&reads, &root, CLAIM_REGISTRY_PATH, MAX_CONTRACT_BYTES)?;
    let surfaces = read_required(&reads, &root, SURFACE_CATALOG_PATH, MAX_CONTRACT_BYTES)?;
    let facts = facts(&catalog, &contract, &generated, &claims, &surfaces)?;
    let brief = read_optional(&reads, &root, BRIEF_PATH, MAX_BRIEF_BYTES)?;
    reads.revalidate().map_err(InceptionError::context)?;
    context.revalidate().map_err(InceptionError::context)?;
    let candidate = candidate(context, &reads)?;
    Ok(ReadResult {
        catalog,
        facts,
        candidate,
        brief,
    })
}
fn candidate(
    context: &LiveContext,
    reads: &ReadSession,
) -> Result<CandidateBinding, InceptionError> {
    let subject = inception_subject_identity(reads).map_err(InceptionError::context)?;
    Ok(CandidateBinding {
        head_commit: context.candidate().head_commit.clone(),
        head_tree: context.candidate().head_tree.clone(),
        branch: context.candidate().branch.clone(),
        dirty: context.candidate().dirty,
        subject_dirty: subject.dirty,
        candidate_digest: subject.digest,
        repository_digest: digest::bytes(context.roots().repository_root.as_bytes()),
    })
}
fn facts(
    catalog: &AuthorityCatalog,
    contract: &[u8],
    generated: &[u8],
    claims: &[u8],
    surfaces: &[u8],
) -> Result<ContractFacts, InceptionError> {
    let contract_value = parse_json(contract, "contract_json_invalid")?;
    let generated_value = parse_json(generated, "generated_contract_authority_invalid")?;
    let claim_value = parse_json(claims, "claim_registry_invalid")?;
    let surface_value = parse_json(surfaces, "surface_catalog_invalid")?;
    let product_contract_id = text(&contract_value, "product_success_contract_id")?;
    let contract_version = text(&contract_value, "contract_version")?;
    if !product_contract_version_supported(&contract_version) {
        return Err(InceptionError::ContractBindingInvalid);
    }
    let authority_contract_id = text(&claim_value, "contract_id")?;
    let contract_claim_ids = string_array(&contract_value, "claim_ids")?;
    let claim_ids = rows(&claim_value, "claims")?
        .iter()
        .map(|row| text(row, "claim_id"))
        .collect::<Result<Vec<_>, _>>()?;
    let surface_contract_id = text(&surface_value, "contract_id")?;
    let surface_ids = rows(&surface_value, "surfaces")?
        .iter()
        .map(|row| text(row, "surface_id"))
        .collect::<Result<Vec<_>, _>>()?;
    if authority_contract_id != catalog.contract_id()
        || surface_contract_id != authority_contract_id
        || !same_set(&contract_claim_ids, &claim_ids)
        || !unique(&contract_claim_ids)
        || !unique(&claim_ids)
        || !unique(&surface_ids)
    {
        return Err(InceptionError::ContractBindingInvalid);
    }
    let expected_contract_digest = generated_surface_digest(&generated_value)?;
    let contract_digest = digest::bytes(contract);
    if contract_digest != expected_contract_digest
        || !catalog_has_digest(catalog, GENERATED_AUTHORITY_PATH, &digest::bytes(generated))
        || !catalog_has_digest(catalog, CLAIM_REGISTRY_PATH, &digest::bytes(claims))
        || !catalog_has_digest(catalog, SURFACE_CATALOG_PATH, &digest::bytes(surfaces))
    {
        return Err(InceptionError::ContractBindingInvalid);
    }
    Ok(ContractFacts {
        product_contract_id,
        contract_version,
        contract_digest,
        authority_contract_id,
        claim_registry_digest: digest::bytes(claims),
        claim_ids,
        public_surface_catalog_digest: digest::bytes(surfaces),
        surface_ids,
    })
}
pub(super) fn product_contract_version_supported(version: &str) -> bool {
    version == PRODUCT_SUCCESS_CONTRACT_VERSION
}
fn generated_surface_digest(value: &Value) -> Result<String, InceptionError> {
    let row = value
        .get("surfaces")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("output").and_then(Value::as_str) == Some(PRODUCT_CONTRACT_PATH)
            })
        })
        .ok_or(InceptionError::ContractBindingInvalid)?;
    text(row, "sha256").map(|digest| format!("sha256:{digest}"))
}
fn catalog_has_digest(catalog: &AuthorityCatalog, path: &str, digest: &str) -> bool {
    let rows = catalog
        .entries()
        .iter()
        .filter(|entry| entry.relative_path == path)
        .collect::<Vec<_>>();
    rows.len() == 1 && format!("sha256:{}", rows[0].digest_sha256) == digest
}
fn read_required(
    reads: &ReadSession,
    root: &Path,
    relative: &str,
    maximum: u64,
) -> Result<Vec<u8>, InceptionError> {
    let path = confined(root, relative)?;
    regular_file(&path)?;
    reads
        .read_bounded(&path, maximum)
        .map_err(|_| InceptionError::UnsafeInput)
}
fn read_optional(
    reads: &ReadSession,
    root: &Path,
    relative: &str,
    maximum: u64,
) -> Result<Option<Vec<u8>>, InceptionError> {
    let path = confined(root, relative)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            regular_file(&path)?;
            reads
                .read_bounded(&path, maximum)
                .map(Some)
                .map_err(|_| InceptionError::UnsafeInput)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(_) => Err(InceptionError::UnsafeInput),
    }
}
pub(super) fn regular_file(path: &Path) -> Result<(), InceptionError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| InceptionError::UnsafeInput)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_hard_linked(&metadata) {
        return Err(InceptionError::UnsafeInput);
    }
    Ok(())
}
#[cfg(unix)]
fn is_hard_linked(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() != 1
}
#[cfg(not(unix))]
fn is_hard_linked(_metadata: &fs::Metadata) -> bool {
    false
}
pub(super) fn confined(root: &Path, relative: &str) -> Result<PathBuf, InceptionError> {
    let path = root.join(relative);
    if path.strip_prefix(root).is_err()
        || path
            .components()
            .any(|part| part == std::path::Component::ParentDir)
    {
        return Err(InceptionError::UnsafeInput);
    }
    Ok(path)
}
fn parse_json(bytes: &[u8], code: &'static str) -> Result<Value, InceptionError> {
    serde_json::from_slice(bytes).map_err(|_| InceptionError::Code(code))
}
fn rows<'a>(value: &'a Value, key: &'static str) -> Result<&'a [Value], InceptionError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(InceptionError::ContractBindingInvalid)
}
fn string_array(value: &Value, key: &'static str) -> Result<Vec<String>, InceptionError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or(InceptionError::ContractBindingInvalid)?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .filter(|value| !value.is_empty() && value.len() <= 512)
                .map(ToOwned::to_owned)
                .ok_or(InceptionError::ContractBindingInvalid)
        })
        .collect()
}
fn text(value: &Value, key: &'static str) -> Result<String, InceptionError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .map(ToOwned::to_owned)
        .ok_or(InceptionError::Code(key))
}
fn unique(values: &[String]) -> bool {
    !values.is_empty() && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
fn same_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}
