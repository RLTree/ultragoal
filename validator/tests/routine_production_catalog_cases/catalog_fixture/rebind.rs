use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;

use super::custody_types::EntryKind;
use super::directory_entries::{entry_identity, names, open_directory};
use super::scope::{ClaimedFixtureScope, FixtureScopeBinding, FixtureScopeError};

#[cfg(test)]
thread_local! {
    static REFUSALS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OWNER_BOUND_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn set_rebind_refusals(count: usize) {
    REFUSALS.with(|value| value.set(count));
    OWNER_BOUND_SCANS.with(|value| value.set(0));
}

#[cfg(test)]
pub(crate) fn owner_bound_scans() -> usize {
    OWNER_BOUND_SCANS.with(std::cell::Cell::get)
}

pub(crate) fn exact_name(
    parent: &File,
    child: &File,
    device: u64,
    inode: u64,
) -> Result<CString, String> {
    let held = child.metadata().map_err(|error| error.to_string())?;
    if !held.file_type().is_dir() || held.dev() != device || held.ino() != inode {
        return Err("held fixture directory identity changed".to_owned());
    }
    for entry in names(parent.as_raw_fd())? {
        let name = CString::new(entry.into_vec()).map_err(|_| "entry contains NUL".to_owned())?;
        let identity = entry_identity(parent.as_raw_fd(), &name)?;
        if identity.kind != EntryKind::Directory
            || identity.device != device
            || identity.inode != inode
        {
            continue;
        }
        let reopened = open_directory(parent.as_raw_fd(), &name)?;
        let reopened = reopened.metadata().map_err(|error| error.to_string())?;
        if reopened.file_type().is_dir() && reopened.dev() == device && reopened.ino() == inode {
            record_owner_bound_scan();
            if refused() {
                return Err("injected descriptor rebind refusal after owner-bound scan".to_owned());
            }
            return Ok(name);
        }
    }
    Err("held fixture directory has no exact parent binding".to_owned())
}

pub(crate) fn restore(scope: &mut ClaimedFixtureScope) -> Result<(), FixtureScopeError> {
    let name = exact_name(&scope.parent, &scope.directory, scope.device, scope.inode)
        .map_err(FixtureScopeError::Retained)?;
    let Some(parent) = scope.path.parent() else {
        return Err(FixtureScopeError::Retained(
            "descriptor rebind has no recorded parent path".to_owned(),
        ));
    };
    scope.path = parent.join(name.to_string_lossy().as_ref());
    scope.name = name;
    scope.binding = FixtureScopeBinding::Bound;
    Ok(())
}

fn refused() -> bool {
    #[cfg(test)]
    {
        return REFUSALS.with(|value| {
            let count = value.get();
            value.set(count.saturating_sub(1));
            count > 0
        });
    }
    #[cfg(not(test))]
    {
        false
    }
}

fn record_owner_bound_scan() {
    #[cfg(test)]
    OWNER_BOUND_SCANS.with(|value| value.set(value.get().saturating_add(1)));
}
