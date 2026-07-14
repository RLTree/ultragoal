use super::{LegacyCommand, RouteSpec};

const NONE: &[&str] = &[];

pub(super) const ROUTES: &[RouteSpec] = &[
    required(&["law", "graph"], &["--strict"]),
    required(&["law", "check"], &["--gate"]),
    required(&["law", "check"], &["--all"]),
    required(&["standards", "check"], &["--strict"]),
    route(&["capability", "discover"]),
    route(&["capability", "prove"]),
    route(&["install", "audit"]),
    route(&["cache", "audit"]),
    route(&["registry", "probe"]),
    route(&["app-surface", "probe"]),
    route(&["receipts", "verify"]),
    route(&["fixtures", "red"]),
    route(&["fixtures", "green"]),
    route(&["fixtures", "tamper"]),
    route(&["fixtures", "all"]),
    route(&["clean-room", "rebuild"]),
    route(&["claim-ceiling", "compute"]),
    route(&["failure", "capture"]),
    route(&["failure", "promote"]),
    route(&["issue", "check-lifecycle"]),
    route(&["update-goal", "eligibility"]),
    required(&["self", "audit"], &["--strict"]),
    required(&["self", "law-graph"], &["--strict"]),
    route(&["self", "fixtures", "red"]),
    route(&["self", "fixtures", "green"]),
    route(&["self", "fixtures", "tamper"]),
    route(&["self", "update-goal", "eligibility"]),
    route(&["explain"]),
];

const fn route(prefix: &'static [&'static str]) -> RouteSpec {
    RouteSpec {
        prefix,
        required: NONE,
        command: LegacyCommand::Control,
    }
}

const fn required(prefix: &'static [&'static str], required: &'static [&'static str]) -> RouteSpec {
    RouteSpec {
        prefix,
        required,
        command: LegacyCommand::Control,
    }
}
