use super::super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

const PLAN_APIS: &[&str] = &[
    "PackageAction::InstallPlan",
    "HarnessPersonalMarketplaceInstallHandoff-v1",
    "HostPluginRegistryObservation",
];

const VERIFY_APIS: &[&str] = &[
    "PackageAction::InstallVerify",
    "HarnessPersonalMarketplaceInstallHandoff-v1",
    "ReadOnlyTreeObservation",
];

pub(super) const PLAN_BINDING: Binding = binding(
    PublicOperation::PackageInstallPlan,
    SuccessorCommand::Package(PackageAction::InstallPlan),
    EffectClass::Read,
    PLAN_APIS,
);

pub(super) const VERIFY_BINDING: Binding = binding(
    PublicOperation::PackageInstallVerify,
    SuccessorCommand::Package(PackageAction::InstallVerify),
    EffectClass::Read,
    VERIFY_APIS,
);
