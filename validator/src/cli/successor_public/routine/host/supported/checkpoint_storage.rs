use super::super::{
    AnchoredDirectory, CONTINUITY_CHECKPOINT_NAME, CONTINUITY_CHECKPOINT_STAGE_NAME, HostFailure,
    HostState,
};
use super::checkpoint::ContinuationCheckpoint;
use super::continuity_validation::validate_checkpoint;
use std::io::Read;
use std::path::Path;

const MAX_CHECKPOINT_BYTES: u64 = 16 * 1024;

impl HostState {
    pub(crate) fn exact_checkpoint(
        &self,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: Option<&str>,
    ) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
        let Some(checkpoint) = self.read_optional_checkpoint()? else {
            return Ok(None);
        };
        validate_checkpoint(
            &checkpoint,
            target,
            context_id,
            candidate_id,
            plan_id,
            snapshot_id,
            continuation,
        )?;
        Ok(Some(checkpoint))
    }

    pub(super) fn read_optional_checkpoint(
        &self,
    ) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
        if !self
            .adapter
            .entry_names()?
            .iter()
            .any(|name| name == CONTINUITY_CHECKPOINT_NAME)
        {
            return Ok(None);
        }
        let file = self
            .adapter
            .open_regular(CONTINUITY_CHECKPOINT_NAME, libc::O_RDONLY, 0o600)?;
        let mut bytes = Vec::new();
        file.take(MAX_CHECKPOINT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| HostFailure::Invalid)?;
        if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
            return Err(HostFailure::Invalid);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| HostFailure::Invalid)
    }
}

pub(super) fn write_checkpoint(
    adapter: &AnchoredDirectory,
    checkpoint: &ContinuationCheckpoint,
    expected_existing: bool,
) -> Result<(), HostFailure> {
    let bytes = serde_json::to_vec(checkpoint).map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(HostFailure::Invalid);
    }
    adapter.replace_regular_atomically(
        CONTINUITY_CHECKPOINT_NAME,
        CONTINUITY_CHECKPOINT_STAGE_NAME,
        0o600,
        &bytes,
        expected_existing,
    )
}
