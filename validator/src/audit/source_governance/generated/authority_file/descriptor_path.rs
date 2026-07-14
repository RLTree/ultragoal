use super::{AuthorityFileReadError, AuthorityFileReadErrorId, error};
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const MAX_PATH_BYTES: usize = 4096;
const MAX_COMPONENTS: usize = 128;

pub(super) struct DescriptorPath {
    pub(super) absolute: bool,
    pub(super) components: Vec<OsString>,
}

impl DescriptorPath {
    pub(super) fn root(path: &Path) -> Result<Self, AuthorityFileReadError> {
        parse(path, true)
    }

    pub(super) fn relative(path: &Path) -> Result<Self, AuthorityFileReadError> {
        let parsed = parse(path, false)?;
        if parsed.absolute || parsed.components.is_empty() {
            Err(error(AuthorityFileReadErrorId::PathInvalid))
        } else {
            Ok(parsed)
        }
    }

    pub(super) fn relative_path(&self) -> PathBuf {
        self.components.iter().collect()
    }
}

fn parse(path: &Path, root: bool) -> Result<DescriptorPath, AuthorityFileReadError> {
    if path.as_os_str().as_bytes().len() > MAX_PATH_BYTES {
        return Err(error(AuthorityFileReadErrorId::PathInvalid));
    }
    let mut absolute = false;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir if root && components.is_empty() => absolute = true,
            Component::CurDir => {}
            Component::Normal(value) if !value.as_bytes().is_empty() => {
                components.push(value.to_os_string());
            }
            _ => return Err(error(AuthorityFileReadErrorId::PathInvalid)),
        }
    }
    if components.len() > MAX_COMPONENTS || (!root && components.is_empty()) {
        return Err(error(AuthorityFileReadErrorId::PathInvalid));
    }
    Ok(DescriptorPath {
        absolute,
        components,
    })
}
