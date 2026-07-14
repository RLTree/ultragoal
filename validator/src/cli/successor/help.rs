use super::catalog::catalog;
use super::command_contract::{CommandDescriptor, HelpTarget, OutputMode, effect_name};
use std::fmt::Write;

pub const HELP_SCHEMA: &str = "harness-ultragoal.cli-help.v1";
pub const VERSION_SCHEMA: &str = "harness-ultragoal.cli-version.v1";
pub const SUCCESSOR_GRAMMAR_VERSION: &str = "successor-v1-candidate";

pub fn render_help(target: HelpTarget, output_mode: OutputMode) -> String {
    let descriptors: Vec<_> = catalog()
        .iter()
        .filter(|descriptor| match target {
            HelpTarget::Root => true,
            HelpTarget::Group(group) => descriptor.command.group() == group,
            HelpTarget::Command(command) => descriptor.command == command,
        })
        .collect();
    match output_mode {
        OutputMode::Human => human_help(&descriptors),
        OutputMode::Json => json_help(&descriptors),
    }
}

pub fn version_text(output_mode: OutputMode) -> String {
    match output_mode {
        OutputMode::Human => format!("ultragoal {SUCCESSOR_GRAMMAR_VERSION}"),
        OutputMode::Json => format!(
            "{{\"schema_version\":\"{VERSION_SCHEMA}\",\"grammar_version\":\"{SUCCESSOR_GRAMMAR_VERSION}\"}}"
        ),
    }
}

fn human_help(descriptors: &[&CommandDescriptor]) -> String {
    let mut output = String::from(
        "Harness Ultragoal successor CLI\n\
         Usage: ultragoal [--json] <group> [subcommand] [options]\n\
         Effects are structural; no effect override option is accepted.\n\nCommands:\n",
    );
    for descriptor in descriptors {
        let _ = write!(output, "  {}", descriptor.command.group().as_str());
        if let Some(subcommand) = descriptor.subcommand {
            let _ = write!(output, " {subcommand}");
        }
        let _ = write!(output, "  [{}]", effect_name(descriptor.effect));
        for option in descriptor.options {
            let marker = if option.required { "" } else { "[" };
            let close = if option.required { "" } else { "]" };
            let _ = write!(output, " {marker}{}", option.name.as_str());
            if option.kind != super::command_contract::ValueKind::Flag {
                output.push_str(" <value>");
            }
            output.push_str(close);
        }
        let _ = writeln!(output, "\n      {}", descriptor.purpose);
    }
    output
}

fn json_help(descriptors: &[&CommandDescriptor]) -> String {
    let mut output = format!(
        "{{\"schema_version\":\"{HELP_SCHEMA}\",\"grammar_version\":\"{SUCCESSOR_GRAMMAR_VERSION}\",\"commands\":["
    );
    for (index, descriptor) in descriptors.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"group\":\"{}\",\"subcommand\":",
            descriptor.command.group().as_str()
        );
        match descriptor.subcommand {
            Some(subcommand) => write_json_string(&mut output, subcommand),
            None => output.push_str("null"),
        }
        let _ = write!(
            output,
            ",\"effect\":\"{}\",\"purpose\":",
            effect_name(descriptor.effect)
        );
        write_json_string(&mut output, descriptor.purpose);
        output.push_str(",\"options\":[");
        for (option_index, option) in descriptor.options.iter().enumerate() {
            if option_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":\"{}\",\"kind\":\"{}\",\"required\":{}}}",
                option.name.as_str(),
                match option.kind {
                    super::command_contract::ValueKind::Flag => "flag",
                    super::command_contract::ValueKind::Identifier => "identifier",
                    super::command_contract::ValueKind::RelativePath => "relative-path",
                },
                option.required
            );
        }
        output.push_str("]}");
    }
    output.push_str("]}");
    output
}

fn write_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}
