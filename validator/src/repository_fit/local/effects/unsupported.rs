use crate::repository_fit::product_adapter::LocalMutationGrant;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader, error,
};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct LocalEffects;

impl LocalEffects {
    pub(crate) fn open(
        _root: impl AsRef<Path>,
        _unix_modes: BTreeMap<String, u32>,
    ) -> Result<Self, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }

    pub(in crate::repository_fit) fn open_with_mutation_grant(
        _root: impl AsRef<Path>,
        _unix_modes: BTreeMap<String, u32>,
        _grant: LocalMutationGrant,
    ) -> Result<Self, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }

    #[cfg(test)]
    pub(crate) fn open_for_test(
        _root: impl AsRef<Path>,
        _unix_modes: BTreeMap<String, u32>,
    ) -> Result<Self, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }

    pub(crate) fn read_unix_mode(
        &mut self,
        _path: &CanonicalPath,
    ) -> Result<Option<u32>, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }
}

impl FitReader for LocalEffects {
    fn root_binding(&mut self) -> Result<String, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }

    fn read_file(
        &mut self,
        _path: &CanonicalPath,
        _maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }
}

impl FitEffects for LocalEffects {
    fn compare_exchange(
        &mut self,
        _path: &CanonicalPath,
        _expected: &ExpectedContent,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        Err(error(FitErrorId::UnsupportedHost))
    }
}
