use crate::context::LiveContext;
use crate::inventory::AuthorityCatalog;
use crate::state::StateError;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// A narrow, withheld stage for the strict adapter.  It binds the current
/// authority owners and deliberately contains no v2 claim/lane/amendment data.
pub(crate) struct RootClaimStage {
    stage_id: String,
}

#[derive(Serialize)]
struct CurrentAuthorityStage<'a> {
    schema_version: &'static str,
    status: &'static str,
    claim_id: &'static str,
    context_id: &'a str,
    authority_catalog_id: &'a str,
    current_authority_digest: &'a str,
}

pub(in crate::state) fn stage_current_authority(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    authority_digest: String,
) -> Result<RootClaimStage, StateError> {
    context
        .revalidate()
        .map_err(|error| StateError::StaleContext(error.to_string()))?;
    if authority_catalog.context_id() != context.context_id() {
        return Err(StateError::InvalidCatalog(
            "current-authority-catalog-context-mismatch".to_owned(),
        ));
    }
    if !authority_digest.starts_with("sha256:") || authority_digest.len() != 71 {
        return Err(StateError::InvalidCatalog(
            "current-product-authority-identity-invalid".to_owned(),
        ));
    }
    let stage = CurrentAuthorityStage {
        schema_version: "HarnessCurrentAuthorityStage-v1",
        status: "withheld",
        claim_id: "CL-USABLE-LOOP",
        context_id: context.context_id(),
        authority_catalog_id: authority_catalog.catalog_id(),
        current_authority_digest: &authority_digest,
    };
    let bytes = serde_json::to_vec(&stage).map_err(|_| {
        StateError::InvalidCatalog("current-authority-stage-serialization-failed".to_owned())
    })?;
    let stage = RootClaimStage {
        stage_id: format!("sha256:{:x}", Sha256::digest(bytes)),
    };
    context
        .revalidate()
        .map_err(|error| StateError::StaleContext(error.to_string()))?;
    Ok(stage)
}

impl RootClaimStage {
    pub(crate) fn stage_id(&self) -> &str {
        &self.stage_id
    }
}
