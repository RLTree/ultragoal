use super::limits::MAX_REFERENCES;
use super::privacy;
use crate::context::LiveContext;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(super) fn candidate_id(context: &LiveContext) -> Result<String, String> {
    let payload = serde_json::to_vec(context.candidate())
        .map_err(|_| "observe-candidate-binding-failed".to_owned())?;
    Ok(format!("sha256:{:x}", Sha256::digest(payload)))
}

pub(super) fn add_reference(
    set: &mut BTreeSet<String>,
    value: &str,
    label: &str,
) -> Result<(), String> {
    privacy::validate_identifier(&format!("{label}-ref"), value)?;
    if !set.contains(value) && set.len() >= MAX_REFERENCES {
        return Err(format!(
            "observe-{label}-limit: reference cardinality exceeded"
        ));
    }
    set.insert(value.to_owned());
    Ok(())
}
