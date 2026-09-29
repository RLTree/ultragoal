use super::super::super::command_contract::LegacyCommand;

#[path = "control.rs"]
mod control;
#[path = "legacy_routes.rs"]
mod legacy_routes;
#[path = "observe.rs"]
mod observe;
#[path = "../route_spec.rs"]
mod route_spec;

pub(super) use route_spec::RouteSpec;

pub(super) fn classify(tokens: &[&str]) -> Option<LegacyCommand> {
    route_spec::first_match(
        tokens,
        &[observe::ROUTES, control::ROUTES, legacy_routes::ROUTES],
    )
}
