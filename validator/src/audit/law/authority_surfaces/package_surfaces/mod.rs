mod cargo;
mod document;
mod failures;
mod generated;
mod inventory_resource;
mod module_ownership;
mod proof_binding;
mod role;
mod row;
mod rows;
mod source;
pub(crate) use document::InventoryArtifactBinding;
pub(super) use failures::failures;
#[cfg(test)]
pub(crate) use failures::failures_for_test;
pub(super) use rows::{summary_value, value};
pub(super) const CHECK_ID: &str = "purpose-backed-active-files";
