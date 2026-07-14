use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservedDisposition {
    Missing,
    Matching,
    ManagedOutdated,
    Conflict,
}

#[derive(Clone)]
pub(crate) struct ObservedFile {
    pub path: CanonicalPath,
    pub ownership: Ownership,
    pub provenance: OwnershipProvenance,
    pub prior_proof_sha256: Option<String>,
    pub disposition: ObservedDisposition,
    pub observed_sha256: Option<String>,
    pub prior: Option<Vec<u8>>,
}

impl std::fmt::Debug for ObservedFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservedFile")
            .field("path", &self.path)
            .field("ownership", &self.ownership)
            .field("provenance", &self.provenance)
            .field("prior_proof_sha256", &self.prior_proof_sha256)
            .field("disposition", &self.disposition)
            .field("observed_sha256", &self.observed_sha256)
            .field("byte_length", &self.prior.as_ref().map(Vec::len))
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct FitInspection {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) root_binding: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) mode: FitMode,
    pub(crate) classification: RepositoryClass,
    pub(crate) files: Vec<ObservedFile>,
    pub(crate) inspection_sha256: String,
}

impl FitInspection {
    pub fn classification(&self) -> &RepositoryClass {
        &self.classification
    }

    pub fn inspection_sha256(&self) -> &str {
        &self.inspection_sha256
    }

    pub fn root_binding(&self) -> &str {
        &self.root_binding
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "sha256")]
pub enum ExpectedContent {
    Absent,
    ExactDigest(String),
}

#[derive(Clone)]
pub struct Mutation {
    pub(crate) path: CanonicalPath,
    pub(crate) expected: ExpectedContent,
    pub(crate) replacement: Vec<u8>,
    pub(crate) prior: Option<Vec<u8>>,
    pub(crate) ownership: Ownership,
    pub(crate) provenance: OwnershipProvenance,
    pub(crate) prior_proof_sha256: Option<String>,
}

impl Mutation {
    pub fn path(&self) -> &CanonicalPath {
        &self.path
    }

    pub fn expected(&self) -> &ExpectedContent {
        &self.expected
    }

    pub fn replacement_sha256(&self) -> String {
        digest(&self.replacement)
    }
}

impl std::fmt::Debug for Mutation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Mutation")
            .field("path", &self.path)
            .field("expected", &self.expected)
            .field("replacement_sha256", &self.replacement_sha256())
            .field("replacement_byte_length", &self.replacement.len())
            .field("had_prior", &self.prior.is_some())
            .field("ownership", &self.ownership)
            .field("provenance", &self.provenance)
            .field("prior_proof_sha256", &self.prior_proof_sha256)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FitConflict {
    pub(crate) path: CanonicalPath,
    pub(crate) observed_sha256: String,
    pub(crate) ownership: Ownership,
    pub(crate) provenance: OwnershipProvenance,
    pub(crate) prior_proof_sha256: Option<String>,
}

impl FitConflict {
    pub(crate) fn new(
        path: CanonicalPath,
        observed_sha256: String,
        ownership: Ownership,
        provenance: OwnershipProvenance,
        prior_proof_sha256: Option<String>,
    ) -> Self {
        Self {
            path,
            observed_sha256,
            ownership,
            provenance,
            prior_proof_sha256,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RollbackPlan {
    pub(crate) mutation_count: usize,
}

impl RollbackPlan {
    pub const fn mutation_count(&self) -> usize {
        self.mutation_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FitCheck {
    pub(crate) path: CanonicalPath,
    pub(crate) expected: ExpectedContent,
    pub(crate) desired_sha256: String,
    pub(crate) provenance: OwnershipProvenance,
    pub(crate) prior_proof_sha256: Option<String>,
}

impl FitCheck {
    pub fn path(&self) -> &CanonicalPath {
        &self.path
    }

    pub fn expected(&self) -> &ExpectedContent {
        &self.expected
    }

    pub fn desired_sha256(&self) -> &str {
        &self.desired_sha256
    }

    pub fn provenance(&self) -> &OwnershipProvenance {
        &self.provenance
    }

    pub fn prior_proof_sha256(&self) -> Option<&str> {
        self.prior_proof_sha256.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct FitPlan {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) root_binding: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) inspection_sha256: String,
    pub(crate) checks: Vec<FitCheck>,
    pub(crate) mutations: Vec<Mutation>,
    pub(crate) conflicts: Vec<FitConflict>,
    pub(crate) rollback: RollbackPlan,
    pub(crate) plan_sha256: String,
}
