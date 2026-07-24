use super::super::super::{CheckpointBinding, HostFailure};
use super::super::{
    AnchoredDirectory, CONTINUITY_CHECKPOINT_NAME, CONTINUITY_CHECKPOINT_STAGE_NAME,
    CONTINUITY_DIRECTORY_NAME, HostState,
};
use super::checkpoint::ContinuationCheckpoint;
use super::continuity_validation::{
    canonical_record_name, canonical_record_name_for_checkpoint, canonical_stage_name,
    checkpoint_matches_binding, validate_checkpoint, validate_checkpoint_shape,
};
use std::io::Read;

const MAX_CHECKPOINT_BYTES: u64 = 16 * 1024;
const RECORD_PREFIX: &str = "routine-continuation-";
const RECORD_SUFFIX: &str = ".json";

#[derive(Clone, Copy)]
pub(super) enum CheckpointLocation {
    Legacy,
    Canonical,
}

pub(super) struct StoredCheckpoint {
    pub(super) checkpoint: ContinuationCheckpoint,
    pub(super) location: CheckpointLocation,
}

pub(crate) struct ContinuationResolution {
    pub(crate) checkpoint: ContinuationCheckpoint,
    pub(crate) handoff_alias: Option<ContinuationCheckpoint>,
}

impl HostState {
    pub(crate) fn exact_checkpoint(
        &self,
        binding: CheckpointBinding<'_>,
        continuation: Option<&str>,
    ) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
        Ok(
            stored_checkpoint(&self.adapter, binding, continuation)?
                .map(|stored| stored.checkpoint),
        )
    }

    pub(crate) fn resolve_checkpoint(
        &self,
        binding: CheckpointBinding<'_>,
        continuation: &str,
    ) -> Result<Option<ContinuationResolution>, HostFailure> {
        let legacy = read_legacy(&self.adapter, binding, None)?;
        let canonical = read_canonical(&self.adapter, binding, None)?;
        match (legacy, canonical) {
            (Some(legacy), Some(canonical))
                if legacy.continuation == continuation
                    && canonical.continuation != continuation
                    && legacy.state == "reconciled"
                    && canonical.state != "terminal-event-joined" =>
            {
                Ok(Some(ContinuationResolution {
                    checkpoint: canonical,
                    handoff_alias: Some(legacy),
                }))
            }
            (Some(legacy), Some(canonical))
                if canonical.continuation == continuation
                    && legacy.continuation != continuation
                    && canonical.state == "reconciled"
                    && legacy.state != "terminal-event-joined" =>
            {
                Ok(Some(ContinuationResolution {
                    checkpoint: legacy,
                    handoff_alias: Some(canonical),
                }))
            }
            (Some(legacy), None) if legacy.continuation == continuation => {
                Ok(Some(ContinuationResolution {
                    checkpoint: legacy,
                    handoff_alias: None,
                }))
            }
            (None, Some(canonical)) if canonical.continuation == continuation => {
                Ok(Some(ContinuationResolution {
                    checkpoint: canonical,
                    handoff_alias: None,
                }))
            }
            (None, None) => Ok(None),
            _ => Err(HostFailure::Invalid),
        }
    }

    pub(crate) fn remove_alias(
        &self,
        binding: CheckpointBinding<'_>,
        alias: &ContinuationCheckpoint,
    ) -> Result<(), HostFailure> {
        remove_alias(&self.adapter, binding, alias)
    }
}

pub(super) fn stored_checkpoint(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    continuation: Option<&str>,
) -> Result<Option<StoredCheckpoint>, HostFailure> {
    let legacy = read_legacy(adapter, binding, continuation)?;
    let canonical = read_canonical(adapter, binding, continuation)?;
    match (legacy, canonical) {
        (Some(legacy), Some(canonical))
            if legacy.state == "reconciled"
                && canonical.state != "reconciled"
                && legacy.continuation != canonical.continuation =>
        {
            Ok(Some(StoredCheckpoint {
                checkpoint: canonical,
                location: CheckpointLocation::Canonical,
            }))
        }
        (Some(legacy), Some(canonical))
            if canonical.state == "reconciled"
                && legacy.state != "reconciled"
                && legacy.continuation != canonical.continuation =>
        {
            Ok(Some(StoredCheckpoint {
                checkpoint: legacy,
                location: CheckpointLocation::Legacy,
            }))
        }
        (Some(_), Some(_)) => Err(HostFailure::Invalid),
        (Some(checkpoint), None) => Ok(Some(StoredCheckpoint {
            checkpoint,
            location: CheckpointLocation::Legacy,
        })),
        (None, Some(checkpoint)) => Ok(Some(StoredCheckpoint {
            checkpoint,
            location: CheckpointLocation::Canonical,
        })),
        (None, None) => Ok(None),
    }
}

pub(super) fn remove_reconciled_alias(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    active: &ContinuationCheckpoint,
) -> Result<(), HostFailure> {
    let legacy = read_legacy(adapter, binding, None)?;
    if let Some(legacy) = legacy
        && legacy.continuation != active.continuation
        && legacy.state == "reconciled"
    {
        adapter.remove_regular(CONTINUITY_CHECKPOINT_NAME)?;
    }
    let canonical = read_canonical(adapter, binding, None)?;
    if let Some(canonical) = canonical
        && canonical.continuation != active.continuation
        && canonical.state == "reconciled"
    {
        let directory = open_continuations(adapter, false)?.ok_or(HostFailure::Invalid)?;
        directory.remove_regular(&canonical_record_name(binding))?;
    }
    Ok(())
}

pub(super) fn remove_alias(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    alias: &ContinuationCheckpoint,
) -> Result<(), HostFailure> {
    if let Some(current) = read_legacy(adapter, binding, Some(&alias.continuation))?
        && current.generation == alias.generation
    {
        adapter.remove_regular(CONTINUITY_CHECKPOINT_NAME)?;
    }
    if let Some(current) = read_canonical(adapter, binding, Some(&alias.continuation))?
        && current.generation == alias.generation
    {
        let directory = open_continuations(adapter, false)?.ok_or(HostFailure::Invalid)?;
        directory.remove_regular(&canonical_record_name(binding))?;
    }
    Ok(())
}

fn read_legacy(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    continuation: Option<&str>,
) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
    if adapter.stat(CONTINUITY_CHECKPOINT_NAME)?.is_none() {
        return Ok(None);
    }
    let checkpoint = read_checkpoint_file(adapter, CONTINUITY_CHECKPOINT_NAME)?;
    if !checkpoint_matches_binding(&checkpoint, binding) {
        return Ok(None);
    }
    validate_checkpoint(&checkpoint, binding, None)?;
    if continuation.is_some_and(|value| checkpoint.continuation != value) {
        return Ok(None);
    }
    Ok(Some(checkpoint))
}

fn read_canonical(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    continuation: Option<&str>,
) -> Result<Option<ContinuationCheckpoint>, HostFailure> {
    let Some(directory) = open_continuations(adapter, false)? else {
        return Ok(None);
    };
    validate_continuation_directory(&directory)?;
    let mut matching = None;
    for name in directory.entry_names()? {
        let checkpoint = read_checkpoint_file(&directory, &name)?;
        if checkpoint_matches_binding(&checkpoint, binding) {
            validate_checkpoint(&checkpoint, binding, None)?;
            if continuation.is_some_and(|value| checkpoint.continuation != value) {
                continue;
            }
            if matching.replace(checkpoint).is_some() {
                return Err(HostFailure::Invalid);
            }
        }
    }
    directory.verify()?;
    Ok(matching)
}

pub(crate) fn validate_continuation_directory(
    directory: &AnchoredDirectory,
) -> Result<(), HostFailure> {
    for name in directory.entry_names()? {
        validate_record_name(&name)?;
        let checkpoint = read_checkpoint_file(directory, &name)?;
        validate_checkpoint_shape(&checkpoint)?;
        if canonical_record_name_for_checkpoint(&checkpoint)? != name {
            return Err(HostFailure::Invalid);
        }
    }
    Ok(())
}

fn validate_record_name(name: &str) -> Result<(), HostFailure> {
    let Some(digest) = name
        .strip_prefix(RECORD_PREFIX)
        .and_then(|value| value.strip_suffix(RECORD_SUFFIX))
    else {
        return Err(HostFailure::Invalid);
    };
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn read_checkpoint_file(
    directory: &AnchoredDirectory,
    name: &str,
) -> Result<ContinuationCheckpoint, HostFailure> {
    let expected = directory.stat(name)?.ok_or(HostFailure::Invalid)?;
    let file = directory.open_regular(name, libc::O_RDONLY, 0o600)?;
    if super::super::identity(&file.metadata().map_err(|_| HostFailure::Invalid)?) != expected {
        return Err(HostFailure::Invalid);
    }
    let mut bytes = Vec::new();
    file.take(MAX_CHECKPOINT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES || directory.stat(name)? != Some(expected) {
        return Err(HostFailure::Invalid);
    }
    let checkpoint = serde_json::from_slice(&bytes).map_err(|_| HostFailure::Invalid)?;
    directory.verify()?;
    Ok(checkpoint)
}

fn open_continuations(
    adapter: &AnchoredDirectory,
    create: bool,
) -> Result<Option<AnchoredDirectory>, HostFailure> {
    match adapter.open_child(CONTINUITY_DIRECTORY_NAME) {
        Ok(directory) => Ok(Some(directory)),
        Err(HostFailure::Unavailable) if create => adapter
            .open_or_create_owned_child(CONTINUITY_DIRECTORY_NAME)
            .map(|(directory, _)| Some(directory)),
        Err(HostFailure::Unavailable) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn write_checkpoint(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    location: CheckpointLocation,
    checkpoint: &ContinuationCheckpoint,
    expected_existing: bool,
) -> Result<(), HostFailure> {
    let bytes = serde_json::to_vec(checkpoint).map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(HostFailure::Invalid);
    }
    match location {
        CheckpointLocation::Legacy => adapter.replace_regular_atomically(
            CONTINUITY_CHECKPOINT_NAME,
            CONTINUITY_CHECKPOINT_STAGE_NAME,
            0o600,
            &bytes,
            expected_existing,
        ),
        CheckpointLocation::Canonical => {
            let directory = open_continuations(adapter, true)?.ok_or(HostFailure::Invalid)?;
            validate_continuation_directory(&directory)?;
            directory.replace_regular_atomically(
                &canonical_record_name(binding),
                &canonical_stage_name(binding),
                0o600,
                &bytes,
                expected_existing,
            )
        }
    }
}
