use super::successor::command_contract::PackageAction;
use super::successor::{LegacyCommand, ParseOutcome, SuccessorCommand, parse_args};

#[test]
fn legacy_command_classes_return_bounded_guidance_without_selecting_effects() {
    let cases: &[(&[&str], LegacyCommand)] = &[
        (&["archive"], LegacyCommand::Archive),
        (&["audit"], LegacyCommand::Audit),
        (&["law", "graph", "--strict"], LegacyCommand::Control),
        (&["coverage", "prove"], LegacyCommand::Coverage),
        (&["current-state"], LegacyCommand::CurrentState),
        (&["final-packet", "prove"], LegacyCommand::FinalPacket),
        (
            &["red", "fixture", "schedule"],
            LegacyCommand::FixtureSchedule,
        ),
        (
            &["foundational-trace", "check", "--strict"],
            LegacyCommand::FoundationalTrace,
        ),
        (&["gc", "plan"], LegacyCommand::Garbage),
        (&["halo", "capability", "prove"], LegacyCommand::Halo),
        (&["help"], LegacyCommand::Help),
        (
            &["loop", "rust-tests", "impacted"],
            LegacyCommand::ImpactedRustTests,
        ),
        (
            &["improvement-loop", "prove"],
            LegacyCommand::ImprovementLoop,
        ),
        (&["line-caps", "check", "--strict"], LegacyCommand::LineCaps),
        (&["loop", "run"], LegacyCommand::LiveLoop),
        (
            &["mandatory-law", "validation"],
            LegacyCommand::MandatoryLawValidation,
        ),
        (
            &["namespace", "check", "--strict"],
            LegacyCommand::Namespace,
        ),
        (&["observe", "logs", "query"], LegacyCommand::Observe),
        (&["openai", "config", "prove"], LegacyCommand::OpenAi),
        (&["package", "digest"], LegacyCommand::PackageDigest),
        (&["performance", "prove"], LegacyCommand::Performance),
        (&["product", "prove-cohesion"], LegacyCommand::Product),
        (&["promptfoo", "prove"], LegacyCommand::Promptfoo),
        (&["red-fixture-report"], LegacyCommand::RedReport),
        (&["review-round"], LegacyCommand::ReviewRound),
        (&["review-target"], LegacyCommand::ReviewTarget),
        (&["routine", "check"], LegacyCommand::Routine),
        (&["rust", "fast"], LegacyCommand::Rust),
        (&["schema", "validation"], LegacyCommand::SchemaValidation),
        (&["semantic-receipts"], LegacyCommand::SemanticReceipts),
        (
            &["session-log", "hardening", "rebind"],
            LegacyCommand::Session,
        ),
        (
            &["source-obligations", "check", "--strict"],
            LegacyCommand::SourceObligations,
        ),
        (
            &["transaction", "finalize"],
            LegacyCommand::TransactionalFinalization,
        ),
        (
            &["typed-boundaries", "check", "--strict"],
            LegacyCommand::TypedBoundaries,
        ),
    ];
    for (args, expected) in cases {
        let outcome = parse_args(args.iter().copied()).expect("legacy intent is bounded");
        assert!(
            matches!(outcome, ParseOutcome::Compatibility { command, .. } if command == *expected),
            "{args:?}: {outcome:?}"
        );
    }
}

#[test]
fn retired_skill_aliases_are_not_public_compatibility_routes() {
    for args in [
        &["agent-first-repo-init", "prove"][..],
        &["agent-first-repo-retrofit", "prove"][..],
        &["agent-improvement-loop", "prove"][..],
        &["agent-observability-stack", "prove"][..],
        &["agent-runtime-legibility", "prove"][..],
        &["execplan-lane", "prove"][..],
        &["fit-repo", "prove"][..],
        &["harness-engineering", "prove"][..],
        &["orchestrator-reconciler", "prove"][..],
        &["product-cohesion-gate", "prove"][..],
        &["product-fitness-gate", "prove"][..],
        &["proof-gate", "prove"][..],
        &["standards-gardener", "rebind"][..],
        &["ultragoal", "prove"][..],
    ] {
        assert!(
            !matches!(
                parse_args(args.iter().copied()),
                Ok(ParseOutcome::Compatibility { .. })
            ),
            "retired skill alias remained a public route: {args:?}"
        );
    }
}

#[test]
fn exact_successor_collisions_remain_owned_by_the_successor_catalog() {
    let inventory = parse_args(["package", "inventory", "--output", "out.json"]).unwrap();
    assert!(matches!(
        inventory,
        ParseOutcome::Invocation(invocation)
            if invocation.command == SuccessorCommand::Package(PackageAction::Inventory)
    ));
    let next = parse_args(["next"]).unwrap();
    assert!(matches!(
        next,
        ParseOutcome::Invocation(invocation) if invocation.command == SuccessorCommand::Next
    ));
    for args in [
        &["observe", "query"][..],
        &["package", "build", "--output", "out.json"][..],
        &["--help"][..],
    ] {
        assert!(!matches!(
            parse_args(args.iter().copied()).unwrap(),
            ParseOutcome::Compatibility { .. }
        ));
    }
}

#[test]
fn legacy_help_and_machine_guidance_never_echo_untrusted_tail_bytes() {
    let outcome = parse_args([
        "--json",
        "current-state",
        "--help",
        "NEVER_ECHO_COMPATIBILITY_CANARY_4819",
    ])
    .unwrap();
    let ParseOutcome::Compatibility {
        command,
        output_mode,
    } = outcome
    else {
        panic!("expected compatibility guidance")
    };
    assert_eq!(command, LegacyCommand::Help);
    let rendered =
        super::successor::compatibility::render_compatibility_guidance(command, output_mode);
    let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(
        value["schema_version"],
        "harness-ultragoal.compatibility-guidance.v1"
    );
    assert_eq!(value["legacy_effect_executed"], false);
    assert_eq!(
        value["exit_code"],
        super::successor::compatibility::COMPATIBILITY_EXIT_CODE
    );
    assert!(!rendered.contains("NEVER_ECHO_COMPATIBILITY_CANARY_4819"));
}
