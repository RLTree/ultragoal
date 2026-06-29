use std::path::Path;

mod compose;
mod files;
mod read;
mod receipt;
mod redaction;
mod registry;

pub(crate) const LAW: &str = "full-local-observability-stack-integration-non-opaque-failure";

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    files::check(root, &mut out);
    registry::check(root, &mut out);
    compose::check(root, &mut out);
    receipt::check(root, &mut out);
    redaction::check(root, &mut out);
    out
}

pub(crate) fn required_commands() -> &'static [&'static str] {
    registry::required_commands()
}
