use crate::audit::observability::registry::command_inventory;
use std::collections::BTreeSet;

#[test]
fn required_command_authority_includes_help_visible_operator_surfaces() {
    let usage = crate::cli::usage::text();
    let required = command_inventory::REQUIRED_COMMANDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for (usage_fragment, command) in [
        ("ultragoal loop run --tier hot", "loop run"),
        ("ultragoal loop measure --node", "loop measure"),
        (
            "ultragoal loop format check --changed-rust",
            "loop format check",
        ),
        ("ultragoal current-state --json", "current-state"),
        ("ultragoal routine check", "routine check"),
        (
            "ultragoal typed-boundaries check --strict",
            "typed-boundaries check",
        ),
        ("ultragoal package inventory", "package inventory"),
        ("ultragoal performance prove", "performance prove"),
        ("ultragoal improvement-loop prove", "improvement-loop prove"),
        ("ultragoal observe fit --command", "observe fit"),
        (
            "ultragoal observe command-roundtrip --command",
            "observe command-roundtrip",
        ),
        ("ultragoal observe explain --next", "observe explain --next"),
    ] {
        assert!(usage.contains(usage_fragment), "{usage_fragment}");
        assert!(required.contains(command), "{command}");
    }
}
