use super::{LegacyCommand, RouteSpec};

const NONE: &[&str] = &[];

pub(super) const ROUTES: &[RouteSpec] = &[
    route(
        &["red", "fixture", "schedule"],
        LegacyCommand::FixtureSchedule,
    ),
    route(&["red", "fixture", "report"], LegacyCommand::RedReport),
    route(
        &["loop", "rust-tests", "impacted"],
        LegacyCommand::ImpactedRustTests,
    ),
    route(&["loop", "format", "check"], LegacyCommand::LiveLoop),
    route(&["loop", "measure"], LegacyCommand::LiveLoop),
    route(&["loop", "run"], LegacyCommand::LiveLoop),
    route(
        &["self", "performance", "prove"],
        LegacyCommand::Performance,
    ),
    route(&["source", "audit"], LegacyCommand::Audit),
    route(&["target-repo", "audit"], LegacyCommand::Audit),
    route(&["final-packet", "prove"], LegacyCommand::FinalPacket),
    route(&["packet", "prove"], LegacyCommand::FinalPacket),
    route(
        &["transaction", "finalize"],
        LegacyCommand::TransactionalFinalization,
    ),
    route(&["package", "digest"], LegacyCommand::PackageDigest),
    route(
        &["mandatory-law", "validation"],
        LegacyCommand::MandatoryLawValidation,
    ),
    route(&["schema", "validation"], LegacyCommand::SchemaValidation),
    route(
        &["session-log", "hardening", "rebind"],
        LegacyCommand::Session,
    ),
    route(&["halo", "capability", "prove"], LegacyCommand::Halo),
    route(&["openai", "config", "prove"], LegacyCommand::OpenAi),
    route(&["openai", "call", "prove"], LegacyCommand::OpenAi),
    route(&["openai", "output", "prove"], LegacyCommand::OpenAi),
    route(&["performance", "prove"], LegacyCommand::Performance),
    route(&["performance", "verify"], LegacyCommand::Performance),
    route(&["performance", "budgets"], LegacyCommand::Performance),
    route(&["product", "prove-cohesion"], LegacyCommand::Product),
    route(&["product", "prove-fitness"], LegacyCommand::Product),
    route(&["product", "prove-journey"], LegacyCommand::Product),
    route(&["rust", "toolchain", "verify"], LegacyCommand::Rust),
    route(&["rust", "memory", "prove"], LegacyCommand::Rust),
    route(&["rust", "dependency", "audit"], LegacyCommand::Rust),
    route(&["rust", "coverage", "prove"], LegacyCommand::Rust),
    route(
        &["rust", "workspace", "topology", "check"],
        LegacyCommand::Rust,
    ),
    route(&["rust", "fast"], LegacyCommand::Rust),
    route(&["rust", "standard"], LegacyCommand::Rust),
    route(&["rust", "release"], LegacyCommand::Rust),
    route(&["rust", "clean-proof"], LegacyCommand::Rust),
    route(&["rust", "watch"], LegacyCommand::Rust),
    route(&["gc", "plan"], LegacyCommand::Garbage),
    route(&["gc", "dry-run"], LegacyCommand::Garbage),
    route(&["gc", "apply"], LegacyCommand::Garbage),
    route(&["gc", "verify"], LegacyCommand::Garbage),
    route(&["coverage", "prove"], LegacyCommand::Coverage),
    required(
        &["foundational-trace", "check"],
        &["--strict"],
        LegacyCommand::FoundationalTrace,
    ),
    required(
        &["line-caps", "check"],
        &["--strict"],
        LegacyCommand::LineCaps,
    ),
    required(
        &["namespace", "check"],
        &["--strict"],
        LegacyCommand::Namespace,
    ),
    required(
        &["source-obligations", "check"],
        &["--strict"],
        LegacyCommand::SourceObligations,
    ),
    required(
        &["typed-boundaries", "check"],
        &["--strict"],
        LegacyCommand::TypedBoundaries,
    ),
    route(&["routine", "check"], LegacyCommand::Routine),
    route(&["schema-validation"], LegacyCommand::SchemaValidation),
    route(
        &["mandatory-law-validation"],
        LegacyCommand::MandatoryLawValidation,
    ),
    route(
        &["improvement-loop", "prove"],
        LegacyCommand::ImprovementLoop,
    ),
    route(&["promptfoo", "prove"], LegacyCommand::Promptfoo),
    route(&["red-fixture-report"], LegacyCommand::RedReport),
    route(&["semantic-receipts"], LegacyCommand::SemanticReceipts),
    route(&["review-round"], LegacyCommand::ReviewRound),
    route(&["review-target"], LegacyCommand::ReviewTarget),
    route(&["current-state"], LegacyCommand::CurrentState),
    route(&["archive"], LegacyCommand::Archive),
    route(&["audit"], LegacyCommand::Audit),
    route(&["help"], LegacyCommand::Help),
];

const fn route(prefix: &'static [&'static str], command: LegacyCommand) -> RouteSpec {
    RouteSpec {
        prefix,
        required: NONE,
        command,
    }
}

const fn required(
    prefix: &'static [&'static str],
    required: &'static [&'static str],
    command: LegacyCommand,
) -> RouteSpec {
    RouteSpec {
        prefix,
        required,
        command,
    }
}
