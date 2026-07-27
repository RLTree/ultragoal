use super::super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

pub(super) const APIS: &[&str] = &[
    "PackageAction::InstallTest",
    "capture_product_package",
    "RuntimeProbePlan::from_installed_package",
    "RuntimeOutcome::payload",
];

pub(super) const BINDING: Binding = binding(
    PublicOperation::PackageInstallTest,
    SuccessorCommand::Package(PackageAction::InstallTest),
    EffectClass::WorkspaceWrite,
    APIS,
);
