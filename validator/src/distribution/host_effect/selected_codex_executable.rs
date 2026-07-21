use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

mod execution;
mod identity;
mod selection;
pub(crate) use selection::SelectedCodexExecutable;

pub(crate) fn resolve_codex_executable() -> Result<SelectedCodexExecutable, DistributionError> {
    let path = env::var_os("PATH").ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if path_byte_length(&path) > MAX_PATH_BYTES {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    resolve_from_path(env::split_paths(&path))
}

const CODEX_PROGRAM: &str = "codex";
const MAX_PATH_BYTES: usize = 64 * 1024;

fn path_byte_length(path: &OsStr) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_bytes().len()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().len()
    }
}

fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<SelectedCodexExecutable, DistributionError> {
    selection::resolve_from_path(paths, CODEX_PROGRAM)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))
}

#[cfg(all(test, unix))]
mod selected_tests;

#[cfg(all(test, unix))]
pub(crate) use selected_tests::{SelectedCodexExecutableTestFixture, selected_test_fixture};
