use super::state::ExpectedContent;
use super::{
    CanonicalPath, FitError, FitErrorId, OwnershipProvenance, digest, error, valid_digest,
};
use serde::Serialize;
use std::collections::BTreeSet;

pub(crate) const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitMode {
    Fresh,
    Retrofit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ownership {
    HarnessGenerated,
    UserOwned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryClass {
    Fresh,
    Partial,
    Conflicting,
    AlreadyFitted,
}

#[derive(Clone)]
pub struct DesiredFile {
    pub(crate) path: CanonicalPath,
    pub(crate) bytes: Vec<u8>,
    pub(crate) ownership: Ownership,
    pub(crate) provenance: OwnershipProvenance,
    pub(crate) known_prior: BTreeSet<String>,
}

impl DesiredFile {
    pub fn user_owned(path: CanonicalPath, bytes: Vec<u8>) -> Result<Self, FitError> {
        Self::build(
            path,
            bytes,
            Ownership::UserOwned,
            OwnershipProvenance::UserDeclared,
            Vec::<String>::new(),
        )
    }

    pub(crate) fn managed(
        path: CanonicalPath,
        bytes: Vec<u8>,
        context_id: String,
        candidate_id: String,
        authority_sha256: String,
        row_sha256: String,
        known_prior: impl IntoIterator<Item = String>,
    ) -> Result<Self, FitError> {
        Self::build(
            path,
            bytes,
            Ownership::HarnessGenerated,
            OwnershipProvenance::adopted(context_id, candidate_id, authority_sha256, row_sha256)?,
            known_prior,
        )
    }

    fn build(
        path: CanonicalPath,
        bytes: Vec<u8>,
        ownership: Ownership,
        provenance: OwnershipProvenance,
        known_prior: impl IntoIterator<Item = String>,
    ) -> Result<Self, FitError> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err(error(FitErrorId::ResourceLimit));
        }
        let known_prior = known_prior.into_iter().collect::<BTreeSet<_>>();
        if known_prior.len() > 32 || known_prior.iter().any(|value| !valid_digest(value)) {
            return Err(error(FitErrorId::InvalidSpec));
        }
        Ok(Self {
            path,
            bytes,
            ownership,
            provenance,
            known_prior,
        })
    }

    pub fn path(&self) -> &CanonicalPath {
        &self.path
    }

    pub fn sha256(&self) -> String {
        digest(&self.bytes)
    }

    pub const fn ownership(&self) -> Ownership {
        self.ownership
    }

    pub fn provenance(&self) -> &OwnershipProvenance {
        &self.provenance
    }
}

impl std::fmt::Debug for DesiredFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DesiredFile")
            .field("path", &self.path)
            .field("sha256", &self.sha256())
            .field("byte_length", &self.bytes.len())
            .field("ownership", &self.ownership)
            .field("provenance", &self.provenance)
            .field("known_prior_count", &self.known_prior.len())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct DesiredState {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) files: Vec<DesiredFile>,
    pub(crate) state_sha256: String,
}

impl DesiredState {
    pub fn new(
        context_id: String,
        candidate_id: String,
        mut files: Vec<DesiredFile>,
    ) -> Result<Self, FitError> {
        if !valid_digest(&context_id)
            || !valid_digest(&candidate_id)
            || files.is_empty()
            || files.len() > MAX_FILES
        {
            return Err(error(FitErrorId::InvalidSpec));
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let mut paths = BTreeSet::new();
        let mut total = 0usize;
        for file in &files {
            total = total
                .checked_add(file.bytes.len())
                .ok_or_else(|| error(FitErrorId::ResourceLimit))?;
            if total > MAX_TOTAL_BYTES
                || !paths.insert(file.path.folded())
                || !file.provenance.valid_for(&context_id, &candidate_id)
                || file.ownership == Ownership::UserOwned
                    && (file.provenance != OwnershipProvenance::UserDeclared
                        || !file.known_prior.is_empty())
                || file.ownership == Ownership::HarnessGenerated
                    && !matches!(file.provenance, OwnershipProvenance::AdoptedManifest { .. })
            {
                return Err(error(if total > MAX_TOTAL_BYTES {
                    FitErrorId::ResourceLimit
                } else {
                    FitErrorId::InvalidSpec
                }));
            }
        }
        #[derive(Serialize)]
        struct Row<'a> {
            path: &'a str,
            sha256: String,
            ownership: Ownership,
            provenance: &'a OwnershipProvenance,
            known_prior: &'a BTreeSet<String>,
        }
        let rows = files
            .iter()
            .map(|file| Row {
                path: file.path.as_str(),
                sha256: file.sha256(),
                ownership: file.ownership,
                provenance: &file.provenance,
                known_prior: &file.known_prior,
            })
            .collect::<Vec<_>>();
        let encoded = serde_json::to_vec(&(&context_id, &candidate_id, rows))
            .map_err(|_| error(FitErrorId::InvalidSpec))?;
        Ok(Self {
            context_id,
            candidate_id,
            files,
            state_sha256: digest(&encoded),
        })
    }

    pub fn state_sha256(&self) -> &str {
        &self.state_sha256
    }
}

pub trait FitReader {
    fn root_binding(&mut self) -> Result<String, FitError>;

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError>;
}

pub trait FitEffects: FitReader {
    /// Compare and replace must be one indivisible effect. `Ok(false)` means
    /// the expected content did not match and the destination was unchanged.
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError>;
}
