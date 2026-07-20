use super::super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

pub(super) const APIS: &[&str] = &["PackageAction::InstallTest", "RuntimeOutcome::failure"];

pub(super) const BINDING: Binding = binding(
    PublicOperation::PackageInstallTest,
    SuccessorCommand::Package(PackageAction::InstallTest),
    EffectClass::WorkspaceWrite,
    APIS,
);
