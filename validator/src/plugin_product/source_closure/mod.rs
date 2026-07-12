mod filesystem;
mod format;
mod model;
mod policy;
mod registry;

pub use model::{
    BuildClosurePolicy, BuildClosureRow, BuildClosureV1, BuildInputKind, ClosureError,
    RequiredBuildInput,
};
pub use registry::plugin_product_build_policy;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

impl BuildClosureV1 {
    pub fn capture(root: &Path, policy: &BuildClosurePolicy) -> Result<Self, ClosureError> {
        Self::capture_with_observer(root, policy, |_, _| {})
    }

    pub fn capture_with_observer<F>(
        root: &Path,
        policy: &BuildClosurePolicy,
        mut after_read: F,
    ) -> Result<Self, ClosureError>
    where
        F: FnMut(usize, &Path),
    {
        policy::validate(policy)?;
        let root = filesystem::checked_root(root)?;
        let mut rows = Vec::with_capacity(policy.required_inputs.len());
        let mut identities = BTreeSet::new();
        let mut total_bytes = 0_u64;
        for (index, input) in policy.required_inputs.iter().enumerate() {
            let relative = filesystem::checked_relative(&input.path)?;
            let path = filesystem::checked_file(&root, &relative)?;
            let (sha256, byte_length, identity) = filesystem::hash_stable(&path)?;
            if !identities.insert(identity) {
                return Err(ClosureError::DuplicateOrAlias);
            }
            total_bytes = bounded_total(total_bytes, byte_length)?;
            rows.push(BuildClosureRow {
                path: input.path.clone(),
                kind: input.kind,
                sha256,
                byte_length,
            });
            after_read(index, &path);
        }
        rows.sort_by(|left, right| left.path.cmp(&right.path));
        let closure = Self {
            schema_version: "BuildClosure-v1".to_owned(),
            aggregate_sha256: format::aggregate(&rows),
            rows,
            final_session_revalidated: true,
        };
        closure
            .verify_at(&root, policy)
            .map_err(|_| ClosureError::FinalSessionDrift)?;
        Ok(closure)
    }

    pub fn verify(&self, root: &Path, policy: &BuildClosurePolicy) -> Result<(), ClosureError> {
        self.verify_at(&filesystem::checked_root(root)?, policy)
    }

    fn verify_at(&self, root: &Path, policy: &BuildClosurePolicy) -> Result<(), ClosureError> {
        policy::validate(policy)?;
        if self.schema_version != "BuildClosure-v1" || !self.final_session_revalidated {
            return Err(ClosureError::FinalSessionDrift);
        }
        if self.rows.len() != policy.required_inputs.len() {
            return Err(ClosureError::UnknownRow);
        }
        let expected = policy
            .required_inputs
            .iter()
            .map(|input| (&input.path, input.kind))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut total_bytes = 0_u64;
        for row in &self.rows {
            format::validate_digest(&row.sha256)?;
            if expected.get(&row.path) != Some(&row.kind) {
                return Err(ClosureError::UnknownRow);
            }
            if !seen.insert(row.path.to_ascii_lowercase()) {
                return Err(ClosureError::DuplicateOrAlias);
            }
            let path = filesystem::checked_file(root, &filesystem::checked_relative(&row.path)?)?;
            let (sha256, byte_length, identity) = filesystem::hash_stable(&path)?;
            if !identities.insert(identity) {
                return Err(ClosureError::DuplicateOrAlias);
            }
            if sha256 != row.sha256 || byte_length != row.byte_length {
                return Err(ClosureError::DigestMismatch);
            }
            total_bytes = bounded_total(total_bytes, byte_length)?;
        }
        if seen.len() != expected.len() {
            return Err(ClosureError::Missing);
        }
        if format::aggregate(&self.rows) != self.aggregate_sha256 {
            return Err(ClosureError::DigestMismatch);
        }
        Ok(())
    }
}

fn bounded_total(current: u64, add: u64) -> Result<u64, ClosureError> {
    let total = current
        .checked_add(add)
        .ok_or(ClosureError::ResourceLimit)?;
    if total > MAX_TOTAL_BYTES {
        return Err(ClosureError::ResourceLimit);
    }
    Ok(total)
}
