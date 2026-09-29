use super::super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::PackageAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

pub(super) const APIS: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "InventoryBuilder",
    "AuthorityCatalog",
    "ProductionPackageSession",
    "PackageSnapshot",
    "ScopedFile",
];

pub(super) const BINDING: Binding = binding(
    PublicOperation::PackageInventory,
    SuccessorCommand::Package(PackageAction::Inventory),
    EffectClass::WorkspaceWrite,
    APIS,
);
