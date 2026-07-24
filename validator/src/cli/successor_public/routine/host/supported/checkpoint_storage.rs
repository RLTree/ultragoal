use super::super::super::{CheckpointBinding, HostFailure};
use super::super::{
    AnchoredDirectory, CONTINUITY_CHECKPOINT_NAME, CONTINUITY_CHECKPOINT_STAGE_NAME,
    CONTINUITY_DIRECTORY_NAME, HostState,
};
use super::checkpoint::ContinuationCheckpoint;
use super::continuity_validation::{
    canonical_record_name, canonical_record_name_for_checkpoint, canonical_stage_name,
    checkpoint_matches_binding, handoff_record_name, handoff_stage_name, validate_checkpoint,
    validate_checkpoint_shape,
};
use std::io::Read;

const MAX_CHECKPOINT_BYTES: u64 = 16 * 1024;
const RECORD_PREFIX: &str = "routine-continuation-";
const HANDOFF_RECORD_PREFIX: &str = "routine-continuation-handoff-";
const RECORD_SUFFIX: &str = ".json";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CheckpointLocation {
    Legacy,
    Canonical,
    Handoff,
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
        let legacy = read_legacy(&self.adapter, binding, Some(continuation))?;
        let canonical = read_canonical(&self.adapter, binding, Some(continuation))?;
        let active = read_canonical(&self.adapter, binding, None)?;
        match (legacy, canonical, active) {
            (Some(legacy), _, Some(active))
                if legacy.continuation == continuation
                    && active.checkpoint.continuation != continuation
                    && legacy.state == "reconciled"
                    && active.checkpoint.state != "terminal-event-joined" =>
            {
                Ok(Some(ContinuationResolution {
                    checkpoint: active.checkpoint,
                    handoff_alias: Some(legacy),
                }))
            }
            (_, Some(canonical), Some(active))
                if canonical.checkpoint.continuation == continuation
                    && active.checkpoint.continuation != continuation
                    && canonical.checkpoint.state == "reconciled"
                    && active.checkpoint.state != "terminal-event-joined" =>
            {
                Ok(Some(ContinuationResolution {
                    checkpoint: active.checkpoint,
                    handoff_alias: Some(canonical.checkpoint),
                }))
            }
            (Some(legacy), _, _) if legacy.continuation == continuation => {
                Ok(Some(ContinuationResolution {
                    checkpoint: legacy,
                    handoff_alias: None,
                }))
            }
            (_, Some(canonical), _) if canonical.checkpoint.continuation == continuation => {
                Ok(Some(ContinuationResolution {
                    checkpoint: canonical.checkpoint,
                    handoff_alias: None,
                }))
            }
            (None, None, _) => Ok(None),
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
                && canonical.checkpoint.state != "reconciled"
                && legacy.continuation != canonical.checkpoint.continuation =>
        {
            Ok(Some(canonical))
        }
        (Some(legacy), Some(canonical))
            if canonical.checkpoint.state == "reconciled"
                && legacy.state != "reconciled"
                && legacy.continuation != canonical.checkpoint.continuation =>
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
        (None, Some(checkpoint)) => Ok(Some(checkpoint)),
        (None, None) => Ok(None),
    }
}

pub(super) fn preserve_reconciled_handoff(
    adapter: &AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    checkpoint: &ContinuationCheckpoint,
) -> Result<(), HostFailure> {
    if checkpoint.state != "reconciled" {
        return Err(HostFailure::Invalid);
    }
    let directory = open_continuations(adapter, true)?.ok_or(HostFailure::Invalid)?;
    validate_continuation_directory(&directory)?;
    if directory.stat(&handoff_record_name(checkpoint)?)?.is_some() {
        return Err(HostFailure::Busy);
    }
    write_checkpoint(
        adapter,
        binding,
        CheckpointLocation::Handoff,
        checkpoint,
        false,
    )
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
    let Some(directory) = open_continuations(adapter, false)? else {
        return Ok(());
    };
    for name in directory.entry_names()? {
        let checkpoint = read_checkpoint_file(&directory, &name)?;
        if checkpoint_matches_binding(&checkpoint, binding)
            && checkpoint.continuation != active.continuation
            && checkpoint.state == "reconciled"
            && checkpoint_location(&name, &checkpoint)? == CheckpointLocation::Handoff
        {
            directory.remove_regular(&name)?;
        }
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
        && current.checkpoint.generation == alias.generation
    {
        let directory = open_continuations(adapter, false)?.ok_or(HostFailure::Invalid)?;
        directory.remove_regular(&record_name(binding, &current)?)?;
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
) -> Result<Option<StoredCheckpoint>, HostFailure> {
    let Some(directory) = open_continuations(adapter, false)? else {
        return Ok(None);
    };
    validate_continuation_directory(&directory)?;
    let mut primary = None;
    let mut matching = None;
    for name in directory.entry_names()? {
        let checkpoint = read_checkpoint_file(&directory, &name)?;
        if checkpoint_matches_binding(&checkpoint, binding) {
            validate_checkpoint(&checkpoint, binding, None)?;
            let location = checkpoint_location(&name, &checkpoint)?;
            if location == CheckpointLocation::Canonical
                && primary
                    .replace(StoredCheckpoint {
                        checkpoint: checkpoint.clone(),
                        location,
                    })
                    .is_some()
            {
                return Err(HostFailure::Invalid);
            }
            if continuation.is_some_and(|value| checkpoint.continuation == value) {
                if matching
                    .replace(StoredCheckpoint {
                        checkpoint,
                        location,
                    })
                    .is_some()
                {
                    return Err(HostFailure::Invalid);
                }
            }
        }
    }
    directory.verify()?;
    Ok(if continuation.is_some() {
        matching
    } else {
        primary
    })
}

pub(crate) fn validate_continuation_directory(
    directory: &AnchoredDirectory,
) -> Result<(), HostFailure> {
    for name in directory.entry_names()? {
        validate_record_name(&name)?;
        let checkpoint = read_checkpoint_file(directory, &name)?;
        validate_checkpoint_shape(&checkpoint)?;
        if checkpoint_location(&name, &checkpoint)? == CheckpointLocation::Handoff
            && checkpoint.state != "reconciled"
        {
            return Err(HostFailure::Invalid);
        }
    }
    Ok(())
}

fn validate_record_name(name: &str) -> Result<(), HostFailure> {
    let Some(digest) = name
        .strip_prefix(HANDOFF_RECORD_PREFIX)
        .or_else(|| name.strip_prefix(RECORD_PREFIX))
        .and_then(|value| value.strip_suffix(RECORD_SUFFIX))
    else {
        return Err(HostFailure::Invalid);
    };
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn checkpoint_location(
    name: &str,
    checkpoint: &ContinuationCheckpoint,
) -> Result<CheckpointLocation, HostFailure> {
    if canonical_record_name_for_checkpoint(checkpoint)? == name {
        Ok(CheckpointLocation::Canonical)
    } else if handoff_record_name(checkpoint)? == name {
        Ok(CheckpointLocation::Handoff)
    } else {
        Err(HostFailure::Invalid)
    }
}

fn record_name(
    binding: CheckpointBinding<'_>,
    stored: &StoredCheckpoint,
) -> Result<String, HostFailure> {
    match stored.location {
        CheckpointLocation::Legacy => Err(HostFailure::Invalid),
        CheckpointLocation::Canonical => Ok(canonical_record_name(binding)),
        CheckpointLocation::Handoff => handoff_record_name(&stored.checkpoint),
    }
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
        CheckpointLocation::Handoff => {
            let directory = open_continuations(adapter, true)?.ok_or(HostFailure::Invalid)?;
            validate_continuation_directory(&directory)?;
            directory.replace_regular_atomically(
                &handoff_record_name(checkpoint)?,
                &handoff_stage_name(checkpoint)?,
                0o600,
                &bytes,
                expected_existing,
            )
        }
    }
}
