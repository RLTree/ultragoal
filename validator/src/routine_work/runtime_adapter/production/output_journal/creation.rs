use super::directory_entries::names;
use super::observation::{open_at, stat_at, validate_directory};
use super::*;

pub(super) enum ProvisionStep {
    Ready(OutputDirectoryIdentity),
    UnrecordedStage(OutputStageAmbiguity),
    StageCreated(OutputDirectoryIdentity),
    StagePresent(OutputDirectoryIdentity),
    Published(OutputDirectoryIdentity),
}

pub(super) fn prepare(
    component: &OutputComponentJournal,
    parent: &File,
    final_name: &str,
    root_device: u64,
) -> Result<ProvisionStep, RoutineError> {
    let nonce = component
        .creation_nonce
        .as_deref()
        .ok_or_else(|| error("routine-production-output-creation-intent-missing"))?;
    let stage_name = format!(".routine-output-{nonce}");
    let final_observed = stat_at(parent, final_name)?;
    let stage_observed = stat_at(parent, &stage_name)?;
    if let Some(expected) = component.provisioned {
        if final_observed != Some(expected) || stage_observed.is_some() {
            return Err(error("routine-production-output-owned-state-changed"));
        }
        validate_exact(parent, final_name, expected, root_device, false)?;
        return Ok(ProvisionStep::Ready(expected));
    }
    match component.staged {
        Some(expected) => {
            return Ok(
                match recover_staged(parent, &stage_name, final_name, expected, root_device)? {
                    StageState::Present(identity) => ProvisionStep::StagePresent(identity),
                    StageState::Published(identity) => ProvisionStep::Published(identity),
                },
            );
        }
        None => {
            if final_observed.is_some() {
                return Err(error("routine-production-output-final-without-custody"));
            }
            if stage_observed.is_some() {
                return Ok(ProvisionStep::UnrecordedStage(OutputStageAmbiguity {
                    relative_path: component.relative_path.clone(),
                    creation_nonce: nonce.to_owned(),
                }));
            }
            let identity = establish_stage(parent, &stage_name, root_device)?;
            return Ok(ProvisionStep::StageCreated(identity));
        }
    }
}

pub(super) fn publish_stage(
    parent: &File,
    component: &OutputComponentJournal,
    final_name: &str,
    expected: OutputDirectoryIdentity,
    root_device: u64,
) -> Result<(), RoutineError> {
    let nonce = component
        .creation_nonce
        .as_deref()
        .ok_or_else(|| error("routine-production-output-creation-intent-missing"))?;
    publish(
        parent,
        &format!(".routine-output-{nonce}"),
        final_name,
        expected,
        root_device,
    )
}

enum StageState {
    Present(OutputDirectoryIdentity),
    Published(OutputDirectoryIdentity),
}

fn establish_stage(
    parent: &File,
    stage_name: &str,
    root_device: u64,
) -> Result<OutputDirectoryIdentity, RoutineError> {
    mkdir_at(parent, stage_name)?;
    parent
        .sync_all()
        .map_err(|_| error("routine-production-output-stage-sync-ambiguous"))?;
    let identity = stat_at(parent, stage_name)?
        .ok_or_else(|| error("routine-production-output-stage-unobserved"))?;
    validate_exact(parent, stage_name, identity, root_device, true)?;
    Ok(identity)
}

fn recover_staged(
    parent: &File,
    stage_name: &str,
    final_name: &str,
    expected: OutputDirectoryIdentity,
    root_device: u64,
) -> Result<StageState, RoutineError> {
    match (stat_at(parent, stage_name)?, stat_at(parent, final_name)?) {
        (Some(stage), None) if stage == expected => {
            validate_exact(parent, stage_name, expected, root_device, true)?;
            Ok(StageState::Present(expected))
        }
        (None, Some(final_identity)) if final_identity == expected => {
            validate_exact(parent, final_name, expected, root_device, false)?;
            Ok(StageState::Published(expected))
        }
        _ => Err(error("routine-production-output-stage-custody-changed")),
    }
}

fn publish(
    parent: &File,
    stage_name: &str,
    final_name: &str,
    expected: OutputDirectoryIdentity,
    root_device: u64,
) -> Result<(), RoutineError> {
    validate_exact(parent, stage_name, expected, root_device, true)?;
    let stage_c = validate_name(stage_name)?;
    let final_c = validate_name(final_name)?;
    let result = unsafe {
        libc::renameatx_np(
            parent.as_raw_fd(),
            stage_c.as_ptr(),
            parent.as_raw_fd(),
            final_c.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result != 0 {
        return Err(error("routine-production-output-publish-conflict"));
    }
    parent
        .sync_all()
        .map_err(|_| error("routine-production-output-publish-sync-ambiguous"))?;
    if stat_at(parent, stage_name)?.is_some() || stat_at(parent, final_name)? != Some(expected) {
        return Err(error("routine-production-output-publish-identity-changed"));
    }
    validate_exact(parent, final_name, expected, root_device, false)
}

fn validate_exact(
    parent: &File,
    name: &str,
    expected: OutputDirectoryIdentity,
    root_device: u64,
    require_empty: bool,
) -> Result<(), RoutineError> {
    validate_directory(expected, root_device)?;
    if expected.owner != unsafe { libc::geteuid() } || expected.mode & 0o7777 != 0o700 {
        return Err(error("routine-production-output-unowned-state"));
    }
    let opened = open_at(parent, name)?;
    if identity(
        &opened
            .metadata()
            .map_err(|_| error("routine-production-output-stat-failed"))?,
    ) != expected
        || stat_at(parent, name)? != Some(expected)
        || require_empty && !names(&opened)?.is_empty()
    {
        return Err(error("routine-production-output-stage-custody-changed"));
    }
    Ok(())
}

fn mkdir_at(parent: &File, name: &str) -> Result<(), RoutineError> {
    let name = validate_name(name)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err(error("routine-production-output-stage-create-failed"));
    }
    Ok(())
}
