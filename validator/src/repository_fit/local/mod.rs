#[cfg(unix)]
mod effects;
#[cfg(unix)]
mod sys;
#[cfg(unix)]
mod unix;

use super::{CanonicalPath, FitError, FitErrorId, FitReader, error};
use std::path::Path;

#[cfg(unix)]
pub(crate) use effects::LocalEffects;

/// Anchored, read-only repository observer. Opening, binding, and reading never
/// create files, locks, directories, receipts, or other workspace state.
pub struct LocalRepository {
    #[cfg(unix)]
    inner: unix::Workspace,
}

impl LocalRepository {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, FitError> {
        #[cfg(unix)]
        {
            return unix::Workspace::open(root.as_ref()).map(|inner| Self { inner });
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            Err(error(FitErrorId::UnsupportedHost))
        }
    }
}

impl FitReader for LocalRepository {
    fn root_binding(&mut self) -> Result<String, FitError> {
        #[cfg(unix)]
        {
            return self.inner.root_binding();
        }
        #[cfg(not(unix))]
        {
            Err(error(FitErrorId::UnsupportedHost))
        }
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        #[cfg(unix)]
        {
            return self.inner.read_file(path, maximum_bytes);
        }
        #[cfg(not(unix))]
        {
            let _ = (path, maximum_bytes);
            Err(error(FitErrorId::UnsupportedHost))
        }
    }
}
