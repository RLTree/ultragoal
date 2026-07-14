use super::*;

impl PinnedFile {
    pub fn bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.initial_bytes)
    }

    pub fn sha256(&self) -> &str {
        &self.initial_sha256
    }

    pub fn byte_length(&self) -> Result<u64, String> {
        self.file
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|_| "file descriptor metadata failed".to_owned())
    }

    pub fn validate(&self, root: &RootAnchor, executable: bool) -> Result<(), String> {
        self.parent.validate(root)?;
        let current = root.open_regular(&self.relative, executable, self.read_limit)?;
        #[cfg(unix)]
        if current.snapshot != self.snapshot
            || current.parent_snapshot != self.parent_snapshot
            || Snapshot::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| "file descriptor metadata failed".to_owned())?,
            ) != self.snapshot
            || Snapshot::from(
                &self
                    .parent
                    .file
                    .metadata()
                    .map_err(|_| "file parent descriptor failed".to_owned())?,
            ) != self.parent_snapshot
        {
            return Err("regular file identity changed during capture".to_owned());
        }
        if current.initial_sha256 != self.initial_sha256 {
            return Err("regular file content changed during capture".to_owned());
        }
        Ok(())
    }
}

pub(crate) fn split_leaf(path: &Path) -> Result<(&Path, &std::ffi::OsStr), String> {
    let name = path
        .file_name()
        .ok_or_else(|| "file path has no final component".to_owned())?;
    Ok((path.parent().unwrap_or_else(|| Path::new(".")), name))
}
