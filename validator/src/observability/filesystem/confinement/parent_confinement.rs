use super::*;

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct VerifiedParent {
    pub(crate) ancestors: Vec<BoundDirectory>,
    pub(crate) name: CString,
}

#[cfg(not(unix))]
#[derive(Debug)]
pub(crate) struct VerifiedParent;

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct BoundDirectory {
    pub(crate) directory: File,
    pub(crate) identity: FileIdentity,
    pub(crate) name_from_parent: Option<CString>,
}

#[cfg(unix)]
pub(crate) fn open_verified_parent(path: &Path) -> Result<VerifiedParent, String> {
    let lexical_parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "observe-store-path-denied: parent is required".to_owned())?;
    let name = path
        .file_name()
        .ok_or_else(|| "observe-store-path-denied: file name is required".to_owned())?;
    let name = CString::new(name.as_bytes())
        .map_err(|_| "observe-store-path-denied: file name is required".to_owned())?;

    // Open each lexical ancestor through a retained directory descriptor. This
    // deliberately avoids canonicalize: canonicalization would follow the very
    // symlink this boundary is supposed to reject.
    let cwd_binding = if lexical_parent.is_absolute() {
        None
    } else {
        let cwd = open_directory(libc::AT_FDCWD, &cstr(b".\0"))?;
        Some(absolute_parent(lexical_parent, directory_identity(&cwd)?)?)
    };
    let parent = cwd_binding
        .as_ref()
        .map_or_else(|| lexical_parent.to_path_buf(), |binding| binding.0.clone());
    let directory = open_directory(libc::AT_FDCWD, &cstr(b"/\0"))?;
    let mut ancestors = vec![BoundDirectory {
        identity: directory_identity(&directory)?,
        directory,
        name_from_parent: None,
    }];
    let mut normal_depth = 0_usize;
    let mut cwd_attached = cwd_binding
        .as_ref()
        .is_none_or(|binding| binding.1 == 0 && ancestors[0].identity == binding.2);
    for component in parent.components() {
        let component = match component {
            Component::RootDir | Component::CurDir => continue,
            Component::ParentDir => cstr(b"..\0"),
            Component::Normal(item) => {
                normal_depth += 1;
                CString::new(item.as_bytes())
                    .map_err(|_| "observe-store-path-denied: parent is required".to_owned())?
            }
            Component::Prefix(_) => {
                return Err("observe-store-path-denied: parent is required".to_owned());
            }
        };
        let directory = open_directory(
            ancestors
                .last()
                .expect("verified parent always has an anchor")
                .directory
                .as_raw_fd(),
            &component,
        )?;
        ancestors.push(BoundDirectory {
            identity: directory_identity(&directory)?,
            directory,
            name_from_parent: Some(component),
        });
        if cwd_binding
            .as_ref()
            .is_some_and(|binding| normal_depth == binding.1)
        {
            cwd_attached = ancestors
                .last()
                .is_some_and(|item| item.identity == cwd_binding.as_ref().unwrap().2);
        }
    }
    if !cwd_attached {
        return Err(
            "observe-store-path-denied: current directory substitution detected".to_owned(),
        );
    }
    Ok(VerifiedParent { ancestors, name })
}

#[cfg(unix)]
impl VerifiedParent {
    pub(crate) fn directory(&self) -> &File {
        &self
            .ancestors
            .last()
            .expect("verified parent always has an anchor")
            .directory
    }

    pub(crate) fn revalidate(&self) -> Result<(), String> {
        for pair in self.ancestors.windows(2) {
            let parent = &pair[0];
            let child = &pair[1];
            validate_directory_link(
                parent.directory.as_raw_fd(),
                child
                    .name_from_parent
                    .as_deref()
                    .expect("non-root anchor has a parent name"),
                child.identity,
            )?;
        }
        Ok(())
    }
}

#[cfg(unix)]
pub(crate) fn inspect_leaf(parent: &VerifiedParent) -> Result<Option<FileIdentity>, String> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: the verified parent retains a live directory descriptor, the
    // leaf name is a NUL-terminated `CString`, and `stat` is writable storage.
    let result = unsafe {
        libc::fstatat(
            parent.directory().as_raw_fd(),
            parent.name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        // SAFETY: a successful `fstatat` initialized the complete stat value.
        let stat = unsafe { stat.assume_init() };
        validate_stat(&stat)?;
        return Ok(Some(identity_stat(&stat)?));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(io_code("metadata", error))
    }
}

#[cfg(unix)]
pub(crate) fn open_leaf(
    parent: &VerifiedParent,
    flags: libc::c_int,
    operation: &str,
) -> Result<File, String> {
    // SAFETY: the verified parent keeps the directory descriptor live, its
    // leaf name is NUL-terminated, and the flags reject symlink traversal.
    let fd = unsafe {
        libc::openat(
            parent.directory().as_raw_fd(),
            parent.name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ELOOP) {
            return Err("observe-store-path-denied: symlink store rejected".to_owned());
        }
        return Err(io_code(operation, error));
    }
    // SAFETY: successful `openat` returns one owned file descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
pub(crate) fn validate_file(
    parent: &VerifiedParent,
    file: &File,
    expected: Option<FileIdentity>,
) -> Result<(), String> {
    parent.revalidate()?;
    let opened = file
        .metadata()
        .map_err(|error| io_code("metadata", error))?;
    validate_metadata(&opened)?;
    let current = inspect_leaf(parent)?
        .ok_or_else(|| "observe-store-path-denied: path substitution detected".to_owned())?;
    let opened_identity = identity(&opened);
    if opened_identity != current || expected.is_some_and(|item| item != opened_identity) {
        return Err("observe-store-path-denied: path substitution detected".to_owned());
    }
    Ok(())
}
