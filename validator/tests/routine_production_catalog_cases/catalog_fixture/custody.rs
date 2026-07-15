use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::custody_types::{EntryIdentity, EntryKind};
use crate::directory_entries::{entry_identity, names, open_directory};

static NEXT_QUARANTINE: AtomicU64 = AtomicU64::new(0);

pub(crate) enum RetainedBinding {
    Named(CString),
    DescriptorOnly,
}

pub(crate) enum CleanupDisposition {
    Deleted,
    Retained {
        binding: RetainedBinding,
        reason: String,
    },
}

pub(crate) fn quarantine_and_remove(
    parent: &File,
    child: &File,
    name: &CStr,
    device: u64,
    inode: u64,
) -> CleanupDisposition {
    let quarantine = match fresh_quarantine(parent.as_raw_fd()) {
        Ok(value) => value,
        Err(error) => return retained(RetainedBinding::Named(copy_name(name)), error),
    };
    if let Err(error) = rename_exclusive(parent.as_raw_fd(), name, &quarantine) {
        return retained(RetainedBinding::Named(copy_name(name)), error);
    }
    if let Err(error) = validate_scope(parent.as_raw_fd(), &quarantine, child, device, inode) {
        return retain_scope(parent, child, name, &quarantine, device, inode, error);
    }
    if let Err(error) = clear(child.as_raw_fd()) {
        return retain_scope(parent, child, name, &quarantine, device, inode, error);
    }
    if let Err(error) = validate_scope(parent.as_raw_fd(), &quarantine, child, device, inode) {
        return retain_scope(parent, child, name, &quarantine, device, inode, error);
    }
    if crate::cleanup_hook::run_before_final_removal(&quarantine) {
        return retain_scope(
            parent,
            child,
            name,
            &quarantine,
            device,
            inode,
            "final removal was retained after the test hook",
        );
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), quarantine.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return retain_scope(
            parent,
            child,
            name,
            &quarantine,
            device,
            inode,
            io::Error::last_os_error().to_string(),
        );
    }
    CleanupDisposition::Deleted
}

fn retain_scope(
    parent: &File,
    child: &File,
    original: &CStr,
    quarantine: &CStr,
    device: u64,
    inode: u64,
    reason: impl ToString,
) -> CleanupDisposition {
    let binding = if validate_scope(parent.as_raw_fd(), quarantine, child, device, inode).is_ok()
        && rename_exclusive(parent.as_raw_fd(), quarantine, original).is_ok()
        && validate_scope(parent.as_raw_fd(), original, child, device, inode).is_ok()
    {
        RetainedBinding::Named(copy_name(original))
    } else if validate_scope(parent.as_raw_fd(), quarantine, child, device, inode).is_ok() {
        RetainedBinding::Named(copy_name(quarantine))
    } else {
        RetainedBinding::DescriptorOnly
    };
    retained(binding, reason)
}

fn clear(directory: RawFd) -> Result<(), String> {
    for entry in names(directory)? {
        let name = CString::new(entry.into_vec()).map_err(|_| "entry contains NUL".to_owned())?;
        let expected = entry_identity(directory, &name)?;
        let quarantine = fresh_quarantine(directory)?;
        rename_exclusive(directory, &name, &quarantine)?;
        validate_entry(directory, &quarantine, expected)?;
        if expected.kind == EntryKind::Directory {
            let child = open_directory(directory, &quarantine)?;
            validate_held(&child, expected)?;
            clear(child.as_raw_fd())?;
            remove_entry(directory, &quarantine, expected, Some(&child))?;
        } else {
            remove_entry(directory, &quarantine, expected, None)?;
        }
    }
    Ok(())
}

fn remove_entry(
    parent: RawFd,
    name: &CStr,
    expected: EntryIdentity,
    held: Option<&File>,
) -> Result<(), String> {
    validate_entry(parent, name, expected)?;
    if let Some(file) = held {
        validate_held(file, expected)?;
    }
    if crate::cleanup_hook::run_before_entry_removal(parent, name) {
        return Err("entry removal was retained after the test hook".to_owned());
    }
    let flags = if expected.kind == EntryKind::Directory {
        libc::AT_REMOVEDIR
    } else {
        0
    };
    if unsafe { libc::unlinkat(parent, name.as_ptr(), flags) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

fn validate_scope(
    parent: RawFd,
    name: &CStr,
    child: &File,
    device: u64,
    inode: u64,
) -> Result<(), String> {
    let named = entry_identity(parent, name)?;
    if named.kind != EntryKind::Directory || named.device != device || named.inode != inode {
        return Err("quarantined scope identity changed".to_owned());
    }
    validate_held(child, named)
}

fn validate_entry(parent: RawFd, name: &CStr, expected: EntryIdentity) -> Result<(), String> {
    if entry_identity(parent, name)? == expected {
        Ok(())
    } else {
        Err("quarantined entry identity changed".to_owned())
    }
}

fn validate_held(file: &File, expected: EntryIdentity) -> Result<(), String> {
    let held = file.metadata().map_err(|error| error.to_string())?;
    if held.dev() == expected.device && held.ino() == expected.inode && held.file_type().is_dir() {
        Ok(())
    } else {
        Err("held directory identity changed".to_owned())
    }
}

fn retained(binding: RetainedBinding, reason: impl ToString) -> CleanupDisposition {
    CleanupDisposition::Retained {
        binding,
        reason: reason.to_string(),
    }
}

fn copy_name(name: &CStr) -> CString {
    CString::new(name.to_bytes()).unwrap_or_default()
}

fn fresh_quarantine(parent: RawFd) -> Result<CString, String> {
    for _ in 0..32 {
        let ordinal = NEXT_QUARANTINE.fetch_add(1, Ordering::Relaxed);
        let candidate = format!(
            ".catalog-fixture-quarantine-{}-{ordinal}",
            std::process::id()
        );
        let name = CString::new(candidate).map_err(|_| "invalid quarantine name".to_owned())?;
        if matches!(entry_identity(parent, &name), Err(error) if error.contains("No such file")) {
            return Ok(name);
        }
    }
    Err("no unused catalog fixture quarantine name".to_owned())
}

#[cfg(target_os = "macos")]
fn rename_exclusive(parent: RawFd, from: &CStr, to: &CStr) -> Result<(), String> {
    if unsafe {
        libc::renameatx_np(
            parent,
            from.as_ptr(),
            parent,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn rename_exclusive(_parent: RawFd, _from: &CStr, _to: &CStr) -> Result<(), String> {
    Err("exclusive rename unavailable on this platform".to_owned())
}
