use std::path::Path;

mod compose;
mod files;
mod read;
mod receipt;
mod redaction;
mod registry;

pub(crate) const LAW: &str = "full-local-observability-stack-integration-non-opaque-failure";
pub(crate) const RECEIPT_SCHEMA: &str = "harness-ultragoal.observability-receipt.v1";

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    files::check(root, &mut out);
    registry::check(root, &mut out);
    compose::check(root, &mut out);
    receipt::check(root, &mut out);
    redaction::check(root, &mut out);
    out
}

#[cfg(test)]
pub(crate) fn required_commands() -> &'static [&'static str] {
    registry::required_commands()
}

#[cfg(test)]
pub(crate) fn required_surfaces() -> &'static [&'static str] {
    registry::required_surfaces()
}

#[cfg(test)]
pub(crate) fn required_loop_stages() -> &'static [&'static str] {
    registry::required_loop_stages()
}

#[cfg(test)]
pub(crate) fn required_signal_classes() -> &'static [&'static str] {
    registry::required_signal_classes()
}

#[cfg(test)]
pub(crate) fn required_dimension_families() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
)> {
    registry::required_dimension_families()
}

pub(crate) fn command_inventory_failures(root: &Path) -> Vec<String> {
    registry::command_inventory_failures(root)
}
