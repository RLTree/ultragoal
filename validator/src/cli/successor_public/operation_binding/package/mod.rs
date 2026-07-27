mod build;
mod install_test;
mod inventory;

use super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

pub(super) const BUILD: Binding = build::BINDING;
pub(super) const INSTALL_TEST: Binding = install_test::BINDING;
pub(super) const INVENTORY: Binding = inventory::BINDING;

const VERIFY_APIS: &[&str] = &[
    "LiveContext::build",
    "InventoryBuilder",
    "ProductionPackageArtifact",
    "PackageSnapshot",
    "ScopedFile",
];

pub(super) const VERIFY: Binding = binding(
    PublicOperation::PackageVerify,
    SuccessorCommand::Package(PackageAction::Verify),
    EffectClass::Read,
    VERIFY_APIS,
);
