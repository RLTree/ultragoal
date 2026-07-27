use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::repository_fit::DesiredState;

use super::catalog::DesiredBundle;
use super::{AdapterErrorId, FitAdapterError, adapter_error, kernel_error};

const ROUTINE_CONFIGURATION_PATHS: [&str; 2] =
    ["config/routine-public.json", "config/routines.json"];

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FitPlanScope {
    #[default]
    CompleteRepository,
    RoutineConfiguration,
    LocalState,
}

impl FitPlanScope {
    pub(crate) const fn includes_local_state(self) -> bool {
        matches!(self, Self::CompleteRepository | Self::LocalState)
    }
}

pub(super) fn select_desired_bundle(
    scope: FitPlanScope,
    bundle: DesiredBundle,
) -> Result<DesiredBundle, FitAdapterError> {
    if scope == FitPlanScope::CompleteRepository {
        return Ok(bundle);
    }
    if scope == FitPlanScope::LocalState {
        return Ok(DesiredBundle {
            desired: DesiredState::local_state_scope(
                bundle.desired.context_id.clone(),
                bundle.desired.candidate_id.clone(),
            )
            .map_err(kernel_error)?,
            authority: bundle.authority,
            unix_modes: Default::default(),
        });
    }
    let paths = ROUTINE_CONFIGURATION_PATHS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let files = bundle
        .desired
        .files
        .iter()
        .filter(|file| paths.contains(file.path.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if files.len() != paths.len() {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    let unix_modes = bundle
        .unix_modes
        .iter()
        .filter(|(path, _)| paths.contains(path.as_str()))
        .map(|(path, mode)| (path.clone(), *mode))
        .collect::<std::collections::BTreeMap<_, _>>();
    if unix_modes.len() != paths.len() {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    let desired = DesiredState::new(
        bundle.desired.context_id.clone(),
        bundle.desired.candidate_id.clone(),
        files,
    )
    .map_err(kernel_error)?;
    Ok(DesiredBundle {
        desired,
        authority: bundle.authority,
        unix_modes,
    })
}

/// Reconstructs the exact scoped desired-state identity during pre-effect
/// revalidation. Local state is intentionally not a template catalog, so its
/// empty file set has a separate, domain-bound constructor.
pub(super) fn rebuild_desired_for_scope(
    scope: FitPlanScope,
    desired: &DesiredState,
) -> Result<DesiredState, FitAdapterError> {
    if scope == FitPlanScope::LocalState {
        if !desired.files.is_empty() {
            return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
        }
        return DesiredState::local_state_scope(
            desired.context_id.clone(),
            desired.candidate_id.clone(),
        )
        .map_err(kernel_error);
    }
    DesiredState::new(
        desired.context_id.clone(),
        desired.candidate_id.clone(),
        desired.files.clone(),
    )
    .map_err(kernel_error)
}
