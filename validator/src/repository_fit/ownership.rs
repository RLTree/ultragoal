#[cfg(test)]
use super::digest;
use super::{CanonicalPath, FitError, FitErrorId, error, valid_digest};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "source")]
pub enum OwnershipProvenance {
    UserDeclared,
    AdoptedManifest {
        context_id: String,
        candidate_id: String,
        authority_sha256: String,
        row_sha256: String,
    },
}

impl OwnershipProvenance {
    pub(crate) fn adopted(
        context_id: String,
        candidate_id: String,
        authority_sha256: String,
        row_sha256: String,
    ) -> Result<Self, FitError> {
        if [&context_id, &candidate_id, &authority_sha256, &row_sha256]
            .into_iter()
            .any(|value| !valid_digest(value))
        {
            return Err(error(FitErrorId::InvalidSpec));
        }
        Ok(Self::AdoptedManifest {
            context_id,
            candidate_id,
            authority_sha256,
            row_sha256,
        })
    }

    pub(crate) fn valid_for(&self, context: &str, candidate: &str) -> bool {
        match self {
            Self::UserDeclared => true,
            Self::AdoptedManifest {
                context_id,
                candidate_id,
                authority_sha256,
                row_sha256,
            } => {
                context_id == context
                    && candidate_id == candidate
                    && valid_digest(authority_sha256)
                    && valid_digest(row_sha256)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedPriorProof {
    context_id: String,
    candidate_id: String,
    root_binding: String,
    path: CanonicalPath,
    current_sha256: String,
    provenance: OwnershipProvenance,
    proof_sha256: String,
}

impl ManagedPriorProof {
    pub fn proof_sha256(&self) -> &str {
        &self.proof_sha256
    }

    pub(crate) fn matches(
        &self,
        context: &str,
        candidate: &str,
        root: &str,
        path: &CanonicalPath,
        current: &str,
        provenance: &OwnershipProvenance,
    ) -> bool {
        self.context_id == context
            && self.candidate_id == candidate
            && self.root_binding == root
            && self.path == *path
            && self.current_sha256 == current
            && self.provenance == *provenance
    }
}

#[cfg(test)]
pub(crate) fn issue_managed_prior_proof(
    context_id: String,
    candidate_id: String,
    root_binding: String,
    path: CanonicalPath,
    current_sha256: String,
    provenance: OwnershipProvenance,
) -> Result<ManagedPriorProof, FitError> {
    if [&context_id, &candidate_id, &root_binding, &current_sha256]
        .into_iter()
        .any(|value| !valid_digest(value))
        || !provenance.valid_for(&context_id, &candidate_id)
        || matches!(provenance, OwnershipProvenance::UserDeclared)
    {
        return Err(error(FitErrorId::InvalidSpec));
    }
    let encoded = serde_json::to_vec(&(
        &context_id,
        &candidate_id,
        &root_binding,
        &path,
        &current_sha256,
        &provenance,
    ))
    .map_err(|_| error(FitErrorId::InvalidSpec))?;
    Ok(ManagedPriorProof {
        context_id,
        candidate_id,
        root_binding,
        path,
        current_sha256,
        provenance,
        proof_sha256: digest(&encoded),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_prior_test_issuer_rejects_user_declared_provenance() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let error = issue_managed_prior_proof(
            digest.clone(),
            digest.clone(),
            digest.clone(),
            CanonicalPath::parse("generated.md").unwrap(),
            digest,
            OwnershipProvenance::UserDeclared,
        )
        .unwrap_err();

        assert_eq!(error.id(), FitErrorId::InvalidSpec);
    }
}
