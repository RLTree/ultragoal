use super::filesystem::{checked_relative, checked_root, read_stable, slash_path};
use super::format::dep_info_tokens;
use super::model::{BuildClosurePolicy, BuildInputKind, ClosureError, RequiredBuildInput};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) const MAX_INPUTS: usize = 8_192;

impl BuildClosurePolicy {
    pub fn new(mut required_inputs: Vec<RequiredBuildInput>) -> Result<Self, ClosureError> {
        if required_inputs.is_empty() || required_inputs.len() > MAX_INPUTS {
            return Err(ClosureError::ResourceLimit);
        }
        for input in &required_inputs {
            checked_relative(&input.path)?;
        }
        required_inputs.sort();
        let mut folded = BTreeSet::new();
        for input in &required_inputs {
            if !folded.insert(input.path.to_ascii_lowercase()) {
                return Err(ClosureError::DuplicateOrAlias);
            }
        }
        Ok(Self {
            schema_version: "HarnessBuildClosurePolicy-v1".to_owned(),
            required_inputs,
        })
    }

    pub fn from_dep_info(
        root: &Path,
        dep_info_paths: &[String],
        dynamic_inputs: Vec<RequiredBuildInput>,
    ) -> Result<Self, ClosureError> {
        let root = checked_root(root)?;
        let mut inputs = BTreeMap::new();
        let mut seen_dep_info = BTreeSet::new();
        for dep_info in dep_info_paths {
            if !seen_dep_info.insert(dep_info.to_ascii_lowercase()) {
                return Err(ClosureError::DuplicateOrAlias);
            }
            let relative = checked_relative(dep_info)?;
            let bytes = read_stable(&root, &relative, 64 * 1024 * 1024)?;
            inputs.insert(dep_info.clone(), BuildInputKind::DepInfo);
            for token in dep_info_tokens(&bytes)? {
                let supplied = if token.is_absolute() {
                    token
                } else {
                    root.path().join(token)
                };
                let canonical = fs::canonicalize(&supplied).map_err(|_| ClosureError::Missing)?;
                let source = canonical
                    .strip_prefix(root.path())
                    .map_err(|_| ClosureError::OutsideRoot)?;
                let value = slash_path(source)?;
                match inputs.get(&value) {
                    Some(BuildInputKind::RustSource) => {}
                    Some(_) => return Err(ClosureError::DuplicateOrAlias),
                    None => {
                        inputs.insert(value, BuildInputKind::RustSource);
                    }
                }
            }
        }
        for input in dynamic_inputs {
            checked_relative(&input.path)?;
            if inputs.insert(input.path, input.kind).is_some() {
                return Err(ClosureError::DuplicateOrAlias);
            }
        }
        Self::new(
            inputs
                .into_iter()
                .map(|(path, kind)| RequiredBuildInput { path, kind })
                .collect(),
        )
    }
}

pub(super) fn validate(policy: &BuildClosurePolicy) -> Result<(), ClosureError> {
    if policy.schema_version != "HarnessBuildClosurePolicy-v1"
        || policy.required_inputs.is_empty()
        || policy.required_inputs.len() > MAX_INPUTS
    {
        return Err(ClosureError::ResourceLimit);
    }
    let rebuilt = BuildClosurePolicy::new(policy.required_inputs.clone())?;
    if rebuilt.required_inputs != policy.required_inputs {
        return Err(ClosureError::DuplicateOrAlias);
    }
    Ok(())
}
