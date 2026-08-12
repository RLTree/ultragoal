use super::InceptionError;
use super::model::{CandidateBinding, CurrentAuthorityFacts};
use crate::context::{LiveContext, ReadSession, inception_subject_identity};
use crate::digest;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const GOAL_CONTRACT_PATH: &str = "GOAL_CONTRACT.md";
pub(crate) const PRODUCT_SUCCESS_CONTRACT_PATH: &str = "PRODUCT_SUCCESS_CONTRACT.md";
pub(crate) const ACTIVE_PLAN_PATH: &str = "docs/exec-plans/active/usable-product-milestone.md";
const CLAIM_ID: &str = "CL-USABLE-LOOP";
const MAX_AUTHORITY_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) struct ReadResult {
    pub(crate) facts: CurrentAuthorityFacts,
    pub(crate) candidate: CandidateBinding,
}

pub(crate) fn read(context: &LiveContext) -> Result<ReadResult, InceptionError> {
    context.revalidate().map_err(InceptionError::context)?;
    let reads = context
        .begin_read_session()
        .map_err(InceptionError::context)?;
    let root = reads.root().to_path_buf();
    let facts = current_authority_facts(&reads, &root)?;
    let candidate = candidate(context, &reads)?;
    reads.revalidate().map_err(InceptionError::context)?;
    context.revalidate().map_err(InceptionError::context)?;
    Ok(ReadResult { facts, candidate })
}

/// Returns a digest over the three current authority owners.  This is the only
/// authority identity admitted to current product state; retained v2 material
/// is verified elsewhere as compatibility context and never reaches this path.
pub(crate) fn current_authority_digest(context: &LiveContext) -> Result<String, InceptionError> {
    let reads = context
        .begin_read_session()
        .map_err(InceptionError::context)?;
    let facts = current_authority_facts(&reads, reads.root())?;
    reads.revalidate().map_err(InceptionError::context)?;
    context.revalidate().map_err(InceptionError::context)?;
    Ok(facts.authority_digest)
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

fn current_authority_facts(
    reads: &ReadSession,
    root: &Path,
) -> Result<CurrentAuthorityFacts, InceptionError> {
    let goal = read_required(reads, root, GOAL_CONTRACT_PATH)?;
    let product = read_required(reads, root, PRODUCT_SUCCESS_CONTRACT_PATH)?;
    require_sole_active_plan(root)?;
    let plan = read_required(reads, root, ACTIVE_PLAN_PATH)?;
    validate_current_authority(&goal, &product, &plan)?;
    let goal_contract_digest = digest::bytes(&goal);
    let product_success_contract_digest = digest::bytes(&product);
    let active_plan_digest = digest::bytes(&plan);
    Ok(CurrentAuthorityFacts {
        claim_id: CLAIM_ID.to_owned(),
        authority_digest: framed_digest([
            (GOAL_CONTRACT_PATH, goal_contract_digest.as_str()),
            (
                PRODUCT_SUCCESS_CONTRACT_PATH,
                product_success_contract_digest.as_str(),
            ),
            (ACTIVE_PLAN_PATH, active_plan_digest.as_str()),
        ]),
        goal_contract_digest,
        product_success_contract_digest,
        active_plan_digest,
    })
}

fn validate_current_authority(
    goal: &[u8],
    product: &[u8],
    plan: &[u8],
) -> Result<(), InceptionError> {
    let goal = std::str::from_utf8(goal).map_err(|_| InceptionError::ContractBindingInvalid)?;
    let product =
        std::str::from_utf8(product).map_err(|_| InceptionError::ContractBindingInvalid)?;
    let plan = std::str::from_utf8(plan).map_err(|_| InceptionError::ContractBindingInvalid)?;
    let goal_plan_owner = format!("The sole executable state owner is\n`{ACTIVE_PLAN_PATH}`.");
    let current_claim = "Harness Ultragoal has one current product claim: `CL-USABLE-LOOP`.";
    if !goal.contains("This contract is the current product-goal authority")
        || occurrences(goal, &goal_plan_owner) != 1
        || occurrences(goal, "## `CL-USABLE-LOOP`") != 1
        || !goal.contains("The external evaluator")
        || !product.contains("This document is the current product-success authority")
        || occurrences(product, current_claim) != 1
        || !plan.starts_with("# Harness UltraGoal")
        || !plan.contains("`CL-USABLE-LOOP`")
        || !plan.contains("The external evaluator")
    {
        return Err(InceptionError::ContractBindingInvalid);
    }
    Ok(())
}

fn require_sole_active_plan(root: &Path) -> Result<(), InceptionError> {
    let directory = root.join("docs/exec-plans/active");
    let metadata = fs::symlink_metadata(&directory).map_err(|_| InceptionError::UnsafeInput)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(InceptionError::UnsafeInput);
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(|_| InceptionError::UnsafeInput)?
        .map(|entry| entry.map_err(|_| InceptionError::UnsafeInput))
        .collect::<Result<Vec<_>, _>>()?;
    if entries.len() != 1
        || entries
            .pop()
            .is_none_or(|entry| entry.file_name() != "usable-product-milestone.md")
    {
        return Err(InceptionError::ContractBindingInvalid);
    }
    Ok(())
}

fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn framed_digest<'a>(items: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"UltraGoalCurrentProductAuthority-v1");
    for (path, digest) in items {
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update((digest.len() as u64).to_be_bytes());
        hasher.update(digest.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn read_required(
    reads: &ReadSession,
    root: &Path,
    relative: &str,
) -> Result<Vec<u8>, InceptionError> {
    let path = confined(root, relative)?;
    regular_file(&path)?;
    reads
        .read_bounded(&path, MAX_AUTHORITY_BYTES)
        .map_err(|_| InceptionError::UnsafeInput)
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
