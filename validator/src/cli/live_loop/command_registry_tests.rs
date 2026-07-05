use super::surfaces::LOOP_VALIDATION_SURFACES;

#[test]
fn loop_command_registry_uses_parseable_product_cli_commands() {
    for surface in LOOP_VALIDATION_SURFACES {
        for command in [surface.canonical_full_command, surface.narrow_rerun] {
            if !command.starts_with("target/debug/ultragoal ") {
                continue;
            }
            let args = command
                .split_whitespace()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>();
            crate::parse_args_from(args)
                .unwrap_or_else(|err| panic!("{} command must parse: {err}", surface.id));
        }
    }
}

#[test]
fn loop_command_registry_rejects_invented_high_frequency_cli_spellings() {
    let rendered = LOOP_VALIDATION_SURFACES
        .iter()
        .flat_map(|surface| {
            [
                surface.command,
                surface.canonical_full_command,
                surface.narrow_rerun,
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["law check --all", "fixtures red", "fixtures all"] {
        assert!(
            !rendered.contains(forbidden),
            "live-loop registry must use product CLI surfaces, not invented `{forbidden}`"
        );
    }
}
