use super::*;

pub(crate) fn normalize_catalog_paths(
    values: Vec<String>,
    maximum: usize,
    reject_overlaps: bool,
    label: &'static str,
) -> CatalogResult<Vec<CatalogPath>> {
    if values.len() > maximum {
        return Err(error(match label {
            "catalog-read-source" => "catalog-read-source-limit-exceeded",
            _ => "catalog-output-scope-limit-exceeded",
        }));
    }
    let mut paths = values
        .into_iter()
        .map(CatalogPath::parse)
        .collect::<CatalogResult<Vec<_>>>()?;
    paths.sort();
    let mut case_paths = BTreeSet::new();
    if paths
        .iter()
        .any(|path| !case_paths.insert(path.as_str().to_ascii_lowercase()))
    {
        return Err(error(match label {
            "catalog-read-source" => "catalog-read-source-ambiguous",
            _ => "catalog-output-scope-ambiguous",
        }));
    }
    if reject_overlaps {
        for (index, left) in paths.iter().enumerate() {
            if paths[index + 1..]
                .iter()
                .any(|right| paths_overlap(left, right))
            {
                return Err(error("catalog-output-scope-overlap"));
            }
        }
    }
    Ok(paths)
}

pub(crate) fn normalize_input_expectations(
    inputs: &mut [TransitiveInputExpectation],
) -> CatalogResult<()> {
    if inputs.is_empty() || inputs.len() > MAX_READ_SOURCES {
        return Err(error("catalog-selection-input-count-invalid"));
    }
    inputs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut case_paths = BTreeSet::new();
    if inputs
        .iter()
        .any(|input| !case_paths.insert(input.relative_path.as_str().to_ascii_lowercase()))
    {
        return Err(error("catalog-selection-input-ambiguous"));
    }
    Ok(())
}

pub(crate) fn paths_overlap(left: &CatalogPath, right: &CatalogPath) -> bool {
    let left = left.as_str().to_ascii_lowercase();
    let right = right.as_str().to_ascii_lowercase();
    left == right
        || right
            .strip_prefix(&left)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || left
            .strip_prefix(&right)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct DirectoryIdentity {
    pub(crate) relative_directory: String,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) link_count: u64,
    pub(crate) byte_length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
    pub(crate) sha256: String,
    pub(crate) ancestors: Vec<DirectoryIdentity>,
}

#[derive(Clone, Debug)]
pub(crate) struct SealedFile {
    pub(crate) root: PathBuf,
    pub(crate) relative: Option<CatalogPath>,
    pub(crate) absolute: PathBuf,
    pub(crate) root_identity: DirectoryIdentity,
    pub(crate) identity: FileIdentity,
    pub(crate) bytes: Vec<u8>,
}

impl SealedFile {
    pub(crate) fn capture_relative(
        root: &Path,
        relative: &CatalogPath,
        maximum: u64,
    ) -> CatalogResult<Self> {
        let root_identity = capture_root(root)?;
        let absolute = root.join(relative.as_str());
        let (identity, bytes) = capture_regular_file(
            root,
            &root_identity,
            &absolute,
            Some(relative),
            maximum,
            true,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            relative: Some(relative.clone()),
            absolute,
            root_identity,
            identity,
            bytes,
        })
    }

    pub(crate) fn verify_current(&self, maximum: u64) -> CatalogResult<()> {
        let current_root = capture_root(&self.root)?;
        if current_root != self.root_identity {
            return Err(error("catalog-root-identity-changed"));
        }
        let (identity, bytes) = capture_regular_file(
            &self.root,
            &self.root_identity,
            &self.absolute,
            self.relative.as_ref(),
            maximum,
            self.relative.is_some(),
        )?;
        let identity_matches = if self.relative.is_some() {
            identity == self.identity
        } else {
            same_runner_file(&identity, &self.identity)
        };
        if !identity_matches || bytes != self.bytes {
            return Err(error(if self.relative.is_some() {
                "catalog-sealed-file-changed"
            } else {
                "catalog-runner-sealed-file-changed"
            }));
        }
        Ok(())
    }

    pub(crate) fn bound_read_source(&self, relative_path: &str) -> BoundReadSource {
        BoundReadSource {
            relative_path: relative_path.to_owned(),
            device: self.identity.device,
            inode: self.identity.inode,
            unix_mode: self.identity.unix_mode,
            owner_user_id: self.identity.owner_user_id,
            owner_group_id: self.identity.owner_group_id,
            link_count: self.identity.link_count,
            byte_length: self.identity.byte_length,
            modified_seconds: self.identity.modified_seconds,
            modified_nanos: self.identity.modified_nanos,
            changed_seconds: self.identity.changed_seconds,
            changed_nanos: self.identity.changed_nanos,
            sha256: self.identity.sha256.clone(),
            ancestors: self.identity.ancestors.clone(),
        }
    }
}
