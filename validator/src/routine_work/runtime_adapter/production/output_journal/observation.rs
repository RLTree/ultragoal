use super::*;

pub(super) fn observe(
    root: &Path,
    scopes: &[RepoPath],
) -> Result<OutputProvisionJournal, RoutineError> {
    let root = open_root(root)?;
    let root_identity = identity(
        &root
            .metadata()
            .map_err(|_| error("routine-production-output-root-stat-failed"))?,
    );
    validate_directory(root_identity, root_identity.device)?;
    let scope_names = canonical_scopes(scopes)?;
    let component_names = component_names(&scope_names);
    let mut components = Vec::with_capacity(component_names.len());
    let mut missing_parent = BTreeSet::new();
    for relative_path in component_names {
        let parent = relative_path
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent);
        let preexisting = if !parent.is_empty() && missing_parent.contains(parent) {
            None
        } else {
            observe_component(&root, &relative_path, root_identity.device)?
        };
        if preexisting.is_none() {
            missing_parent.insert(relative_path.clone());
        }
        components.push(OutputComponentJournal {
            relative_path,
            preexisting,
            provisioned: None,
        });
    }
    Ok(OutputProvisionJournal {
        root: root_identity,
        scopes: scope_names,
        components,
    })
}

pub(super) fn open_root(root: &Path) -> Result<File, RoutineError> {
    if !root.is_absolute() || fs::canonicalize(root).ok().as_deref() != Some(root) {
        return Err(error("routine-production-output-root-invalid"));
    }
    let name = CString::new(root.as_os_str().as_bytes())
        .map_err(|_| error("routine-production-output-root-invalid"))?;
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(error("routine-production-output-root-open-failed"));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn canonical_scopes(scopes: &[RepoPath]) -> Result<Vec<String>, RoutineError> {
    let mut names = scopes
        .iter()
        .map(|scope| scope.as_str().to_owned())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    if names.is_empty() || names.len() != scopes.len() {
        return Err(error("routine-production-output-scopes-invalid"));
    }
    Ok(names)
}

fn component_names(scopes: &[String]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for scope in scopes {
        let mut current = String::new();
        for component in scope.split('/') {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(component);
            names.insert(current.clone());
        }
    }
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort_by(|left, right| {
        left.matches('/')
            .count()
            .cmp(&right.matches('/').count())
            .then_with(|| left.cmp(right))
    });
    names
}

fn observe_component(
    root: &File,
    relative: &str,
    root_device: u64,
) -> Result<Option<OutputDirectoryIdentity>, RoutineError> {
    let mut parent = root
        .try_clone()
        .map_err(|_| error("routine-production-output-root-clone-failed"))?;
    for component in relative.split('/') {
        let Some(observed) = stat_at(&parent, component)? else {
            return Ok(None);
        };
        validate_directory(observed, root_device)?;
        parent = open_at(&parent, component)?;
        if identity(
            &parent
                .metadata()
                .map_err(|_| error("routine-production-output-stat-failed"))?,
        ) != observed
        {
            return Err(error("routine-production-output-observation-raced"));
        }
    }
    Ok(Some(identity(&parent.metadata().map_err(|_| {
        error("routine-production-output-stat-failed")
    })?)))
}

pub(super) fn stat_at(
    parent: &File,
    name: &str,
) -> Result<Option<OutputDirectoryIdentity>, RoutineError> {
    let name = validate_name(name)?;
    let mut value = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        let stat = unsafe { value.assume_init() };
        return Ok(Some(OutputDirectoryIdentity {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            owner: stat.st_uid,
            mode: u32::from(stat.st_mode),
        }));
    }
    if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
        Ok(None)
    } else {
        Err(error("routine-production-output-stat-failed"))
    }
}

pub(super) fn validate_directory(
    value: OutputDirectoryIdentity,
    root_device: u64,
) -> Result<(), RoutineError> {
    if value.device != root_device
        || value.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFDIR)
        || value.owner != unsafe { libc::geteuid() }
        || value.mode & 0o22 != 0
    {
        return Err(error("routine-production-output-component-unsafe"));
    }
    Ok(())
}

pub(super) fn open_at(parent: &File, name: &str) -> Result<File, RoutineError> {
    let name = validate_name(name)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(error("routine-production-output-open-failed"));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
