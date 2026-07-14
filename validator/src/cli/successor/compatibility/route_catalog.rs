use super::super::super::command_contract::LegacyCommand;

#[path = "route_catalog/control.rs"]
mod control;
#[path = "route_catalog/core.rs"]
mod core;
#[path = "route_catalog/observe.rs"]
mod observe;
#[path = "route_spec.rs"]
mod route_spec;

pub(super) use route_spec::RouteSpec;

pub(super) fn classify(tokens: &[&str]) -> Option<LegacyCommand> {
    route_spec::first_match(tokens, &[observe::ROUTES, control::ROUTES, core::ROUTES])
}
