use super::super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

const PLAN_APIS: &[&str] = &[
    "PackageAction::InstallPlan",
    "PersonalMarketplaceInstallPlan",
    "HostPluginRegistryObservation",
];

const APPLY_APIS: &[&str] = &[
    "PackageAction::InstallApply",
    "PersonalMarketplaceInstallPlan",
    "ImmutableAcceptedPlan",
];

pub(super) const PLAN_BINDING: Binding = binding(
    PublicOperation::PackageInstallPlan,
    SuccessorCommand::Package(PackageAction::InstallPlan),
    EffectClass::Read,
    PLAN_APIS,
);

pub(super) const APPLY_BINDING: Binding = binding(
    PublicOperation::PackageInstallApply,
    SuccessorCommand::Package(PackageAction::InstallApply),
    EffectClass::ExternalWrite,
    APPLY_APIS,
);
