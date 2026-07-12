use super::Session;
use crate::package::inventory::anchored::snapshot::Snapshot;
use crate::package::inventory::anchored::sys;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObservedEntry {
    pub(crate) regular: bool,
    pub(crate) directory: bool,
    pub(crate) symlink: bool,
    pub(crate) single_link: bool,
}

impl Session {
    pub(crate) fn verify_root_now(&self) -> Result<(), String> {
        self.verify_root_exact()
    }

    pub(crate) fn seal_root_snapshot(&mut self) -> Result<(), String> {
        self.verify_root_exact()?;
        self.strict_root_snapshot = Some(self.root_snapshot);
        Ok(())
    }

    pub(crate) fn observe_entry(&mut self, relative: &str) -> Result<ObservedEntry, String> {
        self.entries = self.entries.saturating_add(1);
        if self.entries > super::MAX_ENTRIES {
            return Err("anchored package session exceeds its entry limit".to_string());
        }
        let components = sys::components(relative)?;
        let mut parent = self
            .root_file
            .try_clone()
            .map_err(|_| "anchored package root clone failed".to_string())?;
        let mut cursor = PathBuf::new();

        for (index, name) in components.iter().enumerate() {
            cursor.push(name);
            let baseline = sys::nofollow_snapshot(&parent, name)?;
            self.observe(&cursor, baseline)?;
            let leaf = index + 1 == components.len();
            if leaf {
                if baseline.is_directory() {
                    let opened = sys::open_directory(&parent, name)?;
                    super::compare_opened(&opened, baseline, "directory")?;
                }
                self.verify_root_descriptor()?;
                let kind = baseline.entry_kind();
                return Ok(ObservedEntry {
                    regular: matches!(kind, super::super::snapshot::EntryKind::Regular { .. }),
                    directory: matches!(kind, super::super::snapshot::EntryKind::Directory),
                    symlink: matches!(kind, super::super::snapshot::EntryKind::Symlink),
                    single_link: matches!(
                        kind,
                        super::super::snapshot::EntryKind::Regular { single_link: true }
                    ),
                });
            }
            if !baseline.is_directory() {
                return Err("anchored package ancestor is not a directory".to_string());
            }
            let opened = sys::open_directory(&parent, name)?;
            super::compare_opened(&opened, baseline, "directory")?;
            parent = opened;
        }
        Err("anchored package path is empty".to_string())
    }

    fn verify_root_exact(&self) -> Result<(), String> {
        let path = fs::symlink_metadata(&self.root)
            .map_err(|_| "anchored package root final metadata failed".to_string())?;
        if path.file_type().is_symlink() || Snapshot::from_metadata(&path) != self.root_snapshot {
            return Err("anchored package root changed before finalization".to_string());
        }
        let opened = self
            .root_file
            .metadata()
            .map_err(|_| "anchored package root metadata failed".to_string())?;
        if Snapshot::from_metadata(&opened) != self.root_snapshot {
            return Err("anchored package root changed during session".to_string());
        }
        Ok(())
    }

    pub(super) fn root_matches(&self, actual: Snapshot) -> bool {
        self.strict_root_snapshot.map_or_else(
            || actual.same_identity(self.root_snapshot),
            |expected| actual == expected,
        )
    }
}
