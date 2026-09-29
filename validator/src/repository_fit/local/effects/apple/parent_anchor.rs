use super::*;

impl LocalEffects {
    pub(crate) fn parent_anchor(
        &mut self,
        path: &CanonicalPath,
        create: bool,
    ) -> Result<(ParentAnchor, Vec<CreatedDirectory>), FitError> {
        self.verify_root()?;
        let components = path.components().collect::<Vec<_>>();
        let mut directory = duplicate(&self.root)?;
        let mut canonical = self.canonical.clone();
        let mut relative = Vec::<String>::new();
        let mut attachments = Vec::new();
        let mut created = Vec::new();
        let mut budget = EnumerationBudget::new();
        for component in &components[..components.len() - 1] {
            let classification = exact_entry(&directory, component, &mut budget)?;
            match classification {
                EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
                EntryMatch::Absent if !create => return Err(error(FitErrorId::Conflict)),
                EntryMatch::Absent => {
                    let (next, identity) =
                        self.create_directory(&directory, &canonical, component)?;
                    relative.push((*component).to_owned());
                    created.push(CreatedDirectory {
                        path: relative.join("/"),
                        identity,
                    });
                    attachments.push((duplicate(&directory)?, (*component).to_owned(), identity));
                    canonical.push(component);
                    directory = next;
                    continue;
                }
                EntryMatch::Exact => relative.push((*component).to_owned()),
            }
            let before =
                stat_at(&directory, component)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            let next = open_at(
                &directory,
                component,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or_else(|| error(FitErrorId::StaleBinding))?;
            let opened = next.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
            if !opened.is_dir()
                || require_same_device(self.root_identity.device, opened.dev()).is_err()
                || object_identity(&opened) != stat_identity(before)
                || exact_entry(&directory, component, &mut budget)? != EntryMatch::Exact
            {
                return Err(error(if created.is_empty() {
                    FitErrorId::UnsafeObject
                } else {
                    FitErrorId::RollbackFailed
                }));
            }
            attachments.push((
                duplicate(&directory)?,
                (*component).to_owned(),
                object_identity(&opened),
            ));
            canonical.push(component);
            if descriptor_path(&next)? != canonical {
                return Err(error(if created.is_empty() {
                    FitErrorId::StaleBinding
                } else {
                    FitErrorId::RollbackFailed
                }));
            }
            directory = next;
        }
        let anchor = ParentAnchor {
            directory,
            canonical,
            attachments,
        };
        self.verify_parent(&anchor)?;
        Ok((anchor, created))
    }
    pub(crate) fn verify_parent(&self, parent: &ParentAnchor) -> Result<(), FitError> {
        self.verify_root()?;
        let metadata = parent
            .directory
            .metadata()
            .map_err(|_| error(FitErrorId::StaleBinding))?;
        if !metadata.is_dir()
            || require_same_device(self.root_identity.device, metadata.dev()).is_err()
            || descriptor_path(&parent.directory)? != parent.canonical
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        let mut budget = EnumerationBudget::new();
        for (directory, name, expected) in parent.attachments.iter().rev() {
            let stat = stat_at(directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            if exact_entry(directory, name, &mut budget)? != EntryMatch::Exact
                || stat_identity(stat) != *expected
            {
                return Err(error(FitErrorId::StaleBinding));
            }
        }
        Ok(())
    }
    pub(crate) fn create_directory(
        &mut self,
        parent: &File,
        parent_canonical: &Path,
        name: &str,
    ) -> Result<(File, ObjectIdentity), FitError> {
        let staged_name = directory_temp_name()?;
        let mut budget = EnumerationBudget::new();
        if exact_entry(parent, &staged_name, &mut budget)? != EntryMatch::Absent {
            return Err(error(FitErrorId::UnsafeObject));
        }
        mkdir_at(parent, &staged_name, 0o755)?;
        let staged = open_at(
            parent,
            &staged_name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )?
        .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
        let metadata = staged
            .metadata()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        let identity = object_identity(&metadata);
        if !metadata.is_dir()
            || require_same_device(self.root_identity.device, metadata.dev()).is_err()
            || {
                // SAFETY: `staged` owns a live descriptor for the directory created above.
                unsafe { libc::fchmod(staged.as_raw_fd(), 0o755 as libc::mode_t) != 0 }
            }
            || staged.sync_all().is_err()
            || stat_at(parent, &staged_name)?.is_none_or(|stat| stat_identity(stat) != identity)
            || descriptor_path(&staged).ok() != Some(parent_canonical.join(&staged_name))
        {
            return Err(error(FitErrorId::RollbackFailed));
        }
        if let Err(failure) = rename_exclusive(parent, &staged_name, name) {
            let cleaned = self.quarantine_remove_directory_from(
                parent,
                parent_canonical,
                &staged_name,
                identity,
                &staged,
            )?;
            return Err(error(if cleaned {
                failure.id()
            } else {
                FitErrorId::RollbackFailed
            }));
        }
        self.test_after_directory_publish();
        let current = stat_at(parent, name)?;
        if current.is_none_or(|stat| stat_identity(stat) != identity)
            || descriptor_path(&staged).ok() != Some(parent_canonical.join(name))
            || exact_entry(parent, name, &mut budget)? != EntryMatch::Exact
        {
            return Err(error(FitErrorId::RollbackFailed));
        }
        Ok((staged, identity))
    }
}
