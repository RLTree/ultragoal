mod build;
mod install_test;
mod inventory;

use super::Binding;

pub(super) const BUILD: Binding = build::BINDING;
pub(super) const INSTALL_TEST: Binding = install_test::BINDING;
pub(super) const INVENTORY: Binding = inventory::BINDING;
