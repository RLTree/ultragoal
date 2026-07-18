use super::super::super::{FitError, FitErrorId, error};
use super::super::sys::{EntryMatch, EnumerationBudget, PathStat, exact_entry, stat_at};
use super::{Workspace, bounded_read, identity};
use std::ffi::OsString;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

pub(super) enum LeafRead {
    Absent(PathStat),
    Present(PresentLeaf),
}

pub(super) struct PresentLeaf {
    pub(super) bytes: Vec<u8>,
    pub(super) file: File,
    pub(super) expected: PathStat,
    pub(super) parent_expected: PathStat,
}

#[cfg(target_vendor = "apple")]
/// Returns the descriptor's absolute vnode path at one finite `F_GETPATH`
/// observation. Callers bind identity around this point; namespace changes
/// after their final observation are outside the completed read.
pub(super) fn descriptor_path(file: &File) -> Result<PathBuf, FitError> {
    let mut buffer = [0 as libc::c_char; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
        let id = match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EINVAL | libc::ENOTSUP | libc::ENOSYS) => FitErrorId::UnsupportedHost,
            _ => FitErrorId::StaleBinding,
        };
        return Err(error(id));
    }
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| error(FitErrorId::StaleBinding))?;
    let bytes = buffer[..end]
        .iter()
        .map(|byte| *byte as u8)
        .collect::<Vec<_>>();
    if bytes.first() != Some(&b'/') {
        return Err(error(FitErrorId::StaleBinding));
    }
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

#[cfg(not(target_vendor = "apple"))]
pub(super) fn descriptor_path(file: &File) -> Result<PathBuf, FitError> {
    let _ = file;
    Err(error(FitErrorId::UnsupportedHost))
}

pub(super) fn finish_present(
    workspace: &Workspace,
    directory: &File,
    expected_parent: &Path,
    name: &str,
    leaf: PresentLeaf,
    maximum_bytes: usize,
    enumeration_budget: &mut EnumerationBudget,
) -> Result<Option<Vec<u8>>, FitError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (
            workspace,
            directory,
            expected_parent,
            name,
            leaf,
            maximum_bytes,
            enumeration_budget,
        );
        return Err(error(FitErrorId::UnsupportedHost));
    }

    #[cfg(target_vendor = "apple")]
    {
        let PresentLeaf {
            bytes,
            mut file,
            expected,
            parent_expected,
        } = leaf;
        file.seek(SeekFrom::Start(0))
            .map_err(|_| error(FitErrorId::ReadFailed))?;
        let final_bytes = bounded_read(&mut file, maximum_bytes)?;
        if final_bytes != bytes
            || identity(&file.metadata().map_err(|_| error(FitErrorId::ReadFailed))?) != expected
            || identity(
                &directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::ReadFailed))?,
            ) != parent_expected
            || exact_entry(directory, name, enumeration_budget)? != EntryMatch::Exact
            || stat_at(directory, name)? != Some(expected)
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        let expected_path = expected_parent.join(name);
        #[cfg(all(test, target_vendor = "apple"))]
        super::tests::observe(super::tests::ReadEvent::BeforeFinalPathBinding);
        if descriptor_path(directory)? != expected_parent
            || descriptor_path(&file)? != expected_path
            || !active_root_matches(workspace)?
            || active_identity(&expected_path)? != expected
            || identity(
                &file
                    .metadata()
                    .map_err(|_| error(FitErrorId::StaleBinding))?,
            ) != expected
            || identity(
                &directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::StaleBinding))?,
            ) != parent_expected
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        #[cfg(all(test, target_vendor = "apple"))]
        super::tests::observe(super::tests::ReadEvent::AfterFinalPathBinding);
        Ok(Some(bytes))
    }
}

pub(super) fn finish_absent(
    workspace: &Workspace,
    directory: &File,
    expected_parent: &Path,
    name: &str,
    parent_expected: PathStat,
    enumeration_budget: &mut EnumerationBudget,
) -> Result<Option<Vec<u8>>, FitError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (
            workspace,
            directory,
            expected_parent,
            name,
            parent_expected,
            enumeration_budget,
        );
        return Err(error(FitErrorId::UnsupportedHost));
    }

    #[cfg(target_vendor = "apple")]
    {
        match exact_entry(directory, name, enumeration_budget)? {
            EntryMatch::Absent => {}
            EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
            EntryMatch::Exact => return Err(error(FitErrorId::StaleBinding)),
        }
        if stat_at(directory, name)?.is_some()
            || identity(
                &directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::ReadFailed))?,
            ) != parent_expected
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        #[cfg(all(test, target_vendor = "apple"))]
        super::tests::observe(super::tests::ReadEvent::BeforeFinalPathBinding);
        if descriptor_path(directory)? != expected_parent
            || !active_root_matches(workspace)?
            || active_identity(expected_parent)? != parent_expected
            || !matches!(
                std::fs::symlink_metadata(expected_parent.join(name)),
                Err(failure) if failure.kind() == std::io::ErrorKind::NotFound
            )
            || identity(
                &directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::StaleBinding))?,
            ) != parent_expected
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        #[cfg(all(test, target_vendor = "apple"))]
        super::tests::observe(super::tests::ReadEvent::AfterFinalPathBinding);
        Ok(None)
    }
}

#[cfg(target_vendor = "apple")]
fn active_identity(path: &Path) -> Result<PathStat, FitError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| error(FitErrorId::StaleBinding))?;
    if metadata.file_type().is_symlink() {
        return Err(error(FitErrorId::StaleBinding));
    }
    Ok(identity(&metadata))
}

#[cfg(target_vendor = "apple")]
fn active_root_matches(workspace: &Workspace) -> Result<bool, FitError> {
    use std::os::unix::fs::MetadataExt;

    let root = std::fs::symlink_metadata(&workspace.canonical)
        .map_err(|_| error(FitErrorId::StaleBinding))?;
    Ok(!root.file_type().is_symlink()
        && root.is_dir()
        && root.dev() == workspace.device
        && root.ino() == workspace.inode)
}

#[cfg(all(test, not(target_vendor = "apple")))]
mod tests {
    use super::*;

    #[test]
    fn non_apple_descriptor_paths_fail_closed() {
        let directory = File::open("/").unwrap();
        assert_eq!(
            descriptor_path(&directory).unwrap_err().id(),
            FitErrorId::UnsupportedHost
        );
    }
}
