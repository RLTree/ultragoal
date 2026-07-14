use super::{LegacyCommand, RouteSpec};

const NONE: &[&str] = &[];

pub(super) const ROUTES: &[RouteSpec] = &[
    route(&["observe", "stack", "gc", "plan"]),
    route(&["observe", "stack", "gc", "dry-run"]),
    route(&["observe", "stack", "gc", "apply"]),
    route(&["observe", "stack", "up"]),
    route(&["observe", "stack", "health"]),
    route(&["observe", "stack", "smoke"]),
    route(&["observe", "stack", "down"]),
    route(&["observe", "logs", "query"]),
    route(&["observe", "metrics", "query"]),
    route(&["observe", "traces", "query"]),
    route(&["observe", "snapshot"]),
    route(&["observe", "prove"]),
    route(&["observe", "command-roundtrip"]),
    required(&["observe", "explain"], &["--next"]),
    route(&["observe", "explain-failure"]),
    route(&["observe", "explain-claim"]),
    route(&["observe", "explain-check"]),
    route(&["observe", "explain-law"]),
];

const fn route(prefix: &'static [&'static str]) -> RouteSpec {
    RouteSpec {
        prefix,
        required: NONE,
        command: LegacyCommand::Observe,
    }
}

const fn required(prefix: &'static [&'static str], required: &'static [&'static str]) -> RouteSpec {
    RouteSpec {
        prefix,
        required,
        command: LegacyCommand::Observe,
    }
}
