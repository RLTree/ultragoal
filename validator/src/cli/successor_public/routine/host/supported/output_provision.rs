use super::*;

struct CreatedDirectory {
    parent: File,
    name: String,
    identity: Identity,
}

pub(crate) struct OutputProvision {
    created: Vec<CreatedDirectory>,
    committed: bool,
}

impl OutputProvision {
    pub(crate) fn create(target: &Path, node_ids: &[String]) -> Result<Self, HostFailure> {
        let root = open_target(target)?;
        let root_device = identity(&root.metadata().map_err(|_| HostFailure::Invalid)?).device;
        let mut provision = Self {
            created: Vec::new(),
            committed: false,
        };
        let mut nodes = node_ids.to_vec();
        nodes.sort();
        nodes.dedup();
        let result = nodes.iter().try_for_each(|node| {
            validate_name(node)?;
            let mut parent = root.try_clone().map_err(|_| HostFailure::Persistence)?;
            for component in ["target", "routine", node.as_str()] {
                parent = provision.open_or_create(&parent, component, root_device)?;
            }
            Ok(())
        });
        if let Err(error) = result {
            provision.rollback_inner()?;
            return Err(error);
        }
        verify_target(target, &root)?;
        Ok(provision)
    }

    pub(crate) fn commit(mut self) {
        self.committed = true;
    }

    pub(crate) fn rollback(mut self) -> Result<(), HostFailure> {
        self.rollback_inner()?;
        self.committed = true;
        Ok(())
    }

    fn open_or_create(
        &mut self,
        parent: &File,
        name: &str,
        root_device: u64,
    ) -> Result<File, HostFailure> {
        let existed = stat_at(parent, name)?.is_some();
        let mut created_identity = None;
        if !existed {
            mkdir_at(parent, name)?;
            let identity = stat_at(parent, name)?.ok_or(HostFailure::Persistence)?;
            self.created.push(CreatedDirectory {
                parent: parent.try_clone().map_err(|_| HostFailure::Persistence)?,
                name: name.to_owned(),
                identity,
            });
            created_identity = Some(identity);
        }
        let child =
            super::anchored_directory::openat(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let observed = identity(&child.metadata().map_err(|_| HostFailure::Invalid)?);
        if observed.device != root_device
            || observed.owner != unsafe { libc::geteuid() }
            || observed.mode & 0o22 != 0
            || stat_at(parent, name)? != Some(observed)
            || created_identity
                .is_some_and(|created| observed != created || observed.mode & 0o7777 != 0o700)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(child)
    }

    fn rollback_inner(&mut self) -> Result<(), HostFailure> {
        for created in self.created.iter().rev() {
            if stat_at(&created.parent, &created.name)? != Some(created.identity) {
                return Err(HostFailure::Persistence);
            }
            remove_directory_at(&created.parent, &created.name)?;
        }
        self.created.clear();
        Ok(())
    }
}

impl Drop for OutputProvision {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.rollback_inner();
        }
    }
}

fn open_target(target: &Path) -> Result<File, HostFailure> {
    if !target.is_absolute() || fs::canonicalize(target).ok().as_deref() != Some(target) {
        return Err(HostFailure::Invalid);
    }
    let bytes = CString::new(target.as_os_str().as_bytes()).map_err(|_| HostFailure::Invalid)?;
    let fd = unsafe {
        libc::open(
            bytes.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(HostFailure::Unavailable);
    }
    let file = unsafe { File::from_raw_fd(fd) };
    verify_target(target, &file)?;
    Ok(file)
}

fn verify_target(target: &Path, file: &File) -> Result<(), HostFailure> {
    let held = identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
    let named = identity(&fs::symlink_metadata(target).map_err(|_| HostFailure::Invalid)?);
    if !same_anchored_directory(held, named) {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn stat_at(parent: &File, name: &str) -> Result<Option<Identity>, HostFailure> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(Some(stat_identity(&unsafe { metadata.assume_init() })));
    }
    if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
        Ok(None)
    } else {
        Err(HostFailure::Invalid)
    }
}

fn mkdir_at(parent: &File, name: &str) -> Result<(), HostFailure> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn remove_directory_at(parent: &File, name: &str) -> Result<(), HostFailure> {
    let name = CString::new(name).map_err(|_| HostFailure::Persistence)?;
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(HostFailure::Persistence);
    }
    Ok(())
}

#[cfg(test)]
#[path = "output_provision_tests.rs"]
mod tests;
