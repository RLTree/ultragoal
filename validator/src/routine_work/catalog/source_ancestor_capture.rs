use super::*;

#[cfg(unix)]
pub(crate) fn capture_ancestors(
    root: &Path,
    root_identity: &DirectoryIdentity,
    relative: &CatalogPath,
) -> CatalogResult<Vec<DirectoryIdentity>> {
    let mut result = vec![root_identity.clone()];
    let mut current = root.to_path_buf();
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    for (index, component) in components[..components.len().saturating_sub(1)]
        .iter()
        .enumerate()
    {
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| error("catalog-file-ancestor-missing"))?;
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || metadata.dev() != root_identity.device
        {
            return Err(error("catalog-file-ancestor-unsafe"));
        }
        result.push(directory_identity(
            &components[..=index].join("/"),
            &metadata,
        ));
    }
    Ok(result)
}

#[cfg(unix)]
pub(crate) fn capture_absolute_ancestors(path: &Path) -> CatalogResult<Vec<DirectoryIdentity>> {
    let parent = path
        .parent()
        .ok_or_else(|| error("catalog-runner-parent-invalid"))?;
    let mut result = Vec::new();
    let mut current = PathBuf::from("/");
    let root_metadata =
        fs::symlink_metadata(&current).map_err(|_| error("catalog-runner-ancestor-missing"))?;
    result.push(directory_identity("/", &root_metadata));
    for component in parent.components().skip(1) {
        current.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| error("catalog-runner-ancestor-missing"))?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(error("catalog-runner-ancestor-unsafe"));
        }
        let text = current
            .to_str()
            .ok_or_else(|| error("catalog-runner-path-invalid"))?;
        result.push(directory_identity(text, &metadata));
    }
    Ok(result)
}

#[cfg(unix)]
pub(crate) fn validate_output_scope_prefix(
    root: &Path,
    relative: &CatalogPath,
) -> CatalogResult<()> {
    let root_identity = capture_root(root)?;
    let mut current = root.to_path_buf();
    for component in relative.as_str().split('/') {
        current.push(component);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(error("catalog-output-scope-unobservable")),
        };
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || metadata.dev() != root_identity.device
        {
            return Err(error("catalog-output-scope-unsafe"));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn validate_output_scope_prefix(
    _root: &Path,
    _relative: &CatalogPath,
) -> CatalogResult<()> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
pub(crate) fn directory_identity(relative: &str, metadata: &fs::Metadata) -> DirectoryIdentity {
    DirectoryIdentity {
        relative_directory: relative.to_owned(),
        device: metadata.dev(),
        inode: metadata.ino(),
        unix_mode: metadata.mode(),
        owner_user_id: metadata.uid(),
        owner_group_id: metadata.gid(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
pub(crate) fn file_metadata_tuple(
    metadata: &fs::Metadata,
) -> (u64, u64, u32, u32, u32, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.uid(),
        metadata.gid(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[derive(Serialize)]
pub(crate) struct CatalogIdentity<'a> {
    pub(crate) source_sha256: &'a str,
    pub(crate) source_byte_length: u64,
    pub(crate) graph_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) definitions: &'a BTreeMap<String, RoutineDefinition>,
}

#[derive(Serialize)]
pub(crate) struct DefinitionIdentity<'a> {
    pub(crate) definition_id: &'a str,
    pub(crate) node_id: &'a str,
    pub(crate) behavior_id: &'a str,
    pub(crate) depends_on: &'a BTreeSet<String>,
    pub(crate) read_sources: &'a [CatalogPath],
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) output_scopes: &'a [CatalogPath],
    pub(crate) runner_policy: &'static str,
    pub(crate) read_policy: &'static str,
}

impl<'a> From<&'a RoutineDefinition> for DefinitionIdentity<'a> {
    fn from(value: &'a RoutineDefinition) -> Self {
        Self {
            definition_id: &value.definition_id,
            node_id: &value.node_id,
            behavior_id: &value.behavior_id,
            depends_on: &value.depends_on,
            read_sources: &value.read_sources,
            timeout_ms: value.timeout_ms,
            output_budget_bytes: value.output_budget_bytes,
            output_scopes: &value.output_scopes,
            runner_policy: RUNNER_POLICY,
            read_policy: READ_POLICY,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct InvocationIdentity<'a> {
    pub(crate) catalog_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) definition_id: &'a str,
    pub(crate) definition_sha256: &'a str,
    pub(crate) node_id: &'a str,
    pub(crate) behavior_id: &'a str,
    pub(crate) depends_on: &'a [String],
    pub(crate) selected_tool: &'a str,
    pub(crate) selected_tool_identity_sha256: &'a str,
    pub(crate) used_fallback: bool,
    pub(crate) input_id: &'a str,
    pub(crate) program_path_hex: &'a str,
    pub(crate) program_sha256: &'a str,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: u32,
    pub(crate) arguments: &'a [String],
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) read_sources: &'a [BoundReadSource],
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) output_scopes: &'a [BoundOutputScope],
    pub(crate) runner_policy: &'static str,
    pub(crate) read_policy: &'static str,
}
