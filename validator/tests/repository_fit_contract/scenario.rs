use super::repository_fit::{
    CanonicalPath, DesiredFile, DesiredState, ExpectedContent, FitEffects, FitError, FitErrorId,
    FitReader, Ownership,
};
use std::collections::BTreeMap;

pub fn sha(byte: u8) -> String {
    format!("sha256:{}", (byte as char).to_string().repeat(64))
}

pub fn file(path: &str, bytes: &[u8], ownership: Ownership, prior: &[&[u8]]) -> DesiredFile {
    let path = CanonicalPath::parse(path).unwrap();
    match ownership {
        Ownership::UserOwned => DesiredFile::user_owned(path, bytes.to_vec()).unwrap(),
        Ownership::HarnessGenerated => DesiredFile::managed(
            path,
            bytes.to_vec(),
            sha(b'a'),
            sha(b'b'),
            sha(b'f'),
            sha(b'e'),
            prior
                .iter()
                .map(|bytes| super::repository_fit::digest(bytes)),
        )
        .unwrap(),
    }
}

pub fn managed_proof(path: &str, current: &[u8]) -> super::repository_fit::ManagedPriorProof {
    let desired = file(
        path,
        b"managed-successor",
        Ownership::HarnessGenerated,
        &[current],
    );
    super::repository_fit::ownership::issue_managed_prior_proof(
        sha(b'a'),
        sha(b'b'),
        sha(b'c'),
        CanonicalPath::parse(path).unwrap(),
        super::repository_fit::digest(current),
        desired.provenance().clone(),
    )
    .unwrap()
}

pub fn desired(files: Vec<DesiredFile>) -> DesiredState {
    DesiredState::new(sha(b'a'), sha(b'b'), files).unwrap()
}

pub fn authorization(
    plan: &super::repository_fit::FitPlan,
) -> super::repository_fit::PlanAuthorization {
    super::repository_fit::PlanAuthorization::new(sha(b'a'), sha(b'b'), plan.plan_sha256().into())
        .unwrap()
}

#[derive(Default)]
pub struct MemoryRepo {
    pub files: BTreeMap<String, Vec<u8>>,
    pub writes: usize,
    pub reads: usize,
    pub cas_calls: usize,
    pub fail_cas: Option<usize>,
    pub error_after_cas: Option<usize>,
    pub fail_read_after_cas: Option<usize>,
    pub fail_read: Option<usize>,
    pub mutate_cas: Option<usize>,
    pub corrupt_cas: Option<usize>,
    pub external_after_cas: Option<(String, Vec<u8>)>,
    pub external_on_cas: Option<(usize, String, Option<Vec<u8>>)>,
    pub root_swap_on_binding: Option<usize>,
    binding_calls: usize,
    fail_next_read: bool,
}

impl MemoryRepo {
    pub fn with(path: &str, bytes: &[u8]) -> Self {
        let mut value = Self::default();
        value.files.insert(path.into(), bytes.to_vec());
        value
    }

    pub const fn binding_calls(&self) -> usize {
        self.binding_calls
    }

    fn apply_external(&mut self, path: String, replacement: Option<Vec<u8>>) {
        if let Some(bytes) = replacement {
            self.files.insert(path, bytes);
        } else {
            self.files.remove(&path);
        }
    }

    fn binding(&self) -> String {
        if self
            .root_swap_on_binding
            .is_some_and(|at| self.binding_calls >= at)
        {
            sha(b'd')
        } else {
            sha(b'c')
        }
    }
}

impl FitReader for MemoryRepo {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.binding_calls += 1;
        Ok(self.binding())
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        _maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.reads += 1;
        if self.fail_next_read || self.fail_read == Some(self.reads) {
            self.fail_next_read = false;
            return Err(super::repository_fit::error(FitErrorId::ReadFailed));
        }
        Ok(self.files.get(path.as_str()).cloned())
    }
}

impl FitEffects for MemoryRepo {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        self.cas_calls += 1;
        if self.fail_cas == Some(self.cas_calls) {
            return Err(forced_error());
        }
        if self.mutate_cas == Some(self.cas_calls) {
            self.files.insert(path.as_str().into(), b"raced".to_vec());
        }
        let current = self.files.get(path.as_str());
        let matches = match expected {
            ExpectedContent::Absent => current.is_none(),
            ExpectedContent::ExactDigest(expected) => {
                current.is_some_and(|bytes| super::repository_fit::digest(bytes) == *expected)
            }
        };
        if !matches {
            return Ok(false);
        }
        self.writes += 1;
        if let Some(bytes) = replacement {
            self.files.insert(path.as_str().into(), bytes.to_vec());
        } else {
            self.files.remove(path.as_str());
        }
        if self.corrupt_cas == Some(self.cas_calls) {
            self.files
                .insert(path.as_str().into(), b"corrupt-after-cas".to_vec());
        }
        if let Some((path, bytes)) = self.external_after_cas.take() {
            self.files.insert(path, bytes);
        }
        if self
            .external_on_cas
            .as_ref()
            .is_some_and(|(at, _, _)| *at == self.cas_calls)
        {
            let (_, path, replacement) = self.external_on_cas.take().unwrap();
            self.apply_external(path, replacement);
        }
        if self.fail_read_after_cas == Some(self.cas_calls) {
            self.fail_next_read = true;
        }
        if self.error_after_cas == Some(self.cas_calls) {
            return Err(forced_error());
        }
        Ok(true)
    }
}

fn forced_error() -> FitError {
    super::repository_fit::error(FitErrorId::EffectFailed)
}
