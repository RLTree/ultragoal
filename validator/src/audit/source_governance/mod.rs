mod generated;
mod inventory;
pub(crate) mod line_cap;
mod lint_allowance;
mod live_root_document;
pub(crate) mod production_source;
pub(crate) mod rust_syntax;
mod scope;
mod standards_integrity;

#[cfg(test)]
pub(crate) use inventory::capture;
pub(crate) use inventory::{GovernedInventory, GovernedSource, audit};
pub(crate) use line_cap::failures as line_cap_failures;

#[cfg(test)]
mod tests;
