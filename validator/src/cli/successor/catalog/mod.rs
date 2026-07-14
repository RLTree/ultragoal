mod evaluation_and_migration;
mod fitting_and_validation;
mod inspection;
mod observability_and_package;
mod options;

use super::command_contract::{CommandDescriptor, Group};
use std::sync::OnceLock;

const CATALOG_GROUPS: &[&[CommandDescriptor]] = &[
    inspection::COMMANDS,
    fitting_and_validation::COMMANDS,
    observability_and_package::COMMANDS,
    evaluation_and_migration::COMMANDS,
];

static CATALOG: OnceLock<Vec<CommandDescriptor>> = OnceLock::new();

pub fn catalog() -> &'static [CommandDescriptor] {
    CATALOG
        .get_or_init(|| {
            CATALOG_GROUPS
                .iter()
                .flat_map(|commands| commands.iter().copied())
                .collect()
        })
        .as_slice()
}

pub fn descriptor_for(
    group: Group,
    subcommand: Option<&str>,
) -> Option<&'static CommandDescriptor> {
    catalog().iter().find(|descriptor| {
        descriptor.command.group() == group && descriptor.subcommand == subcommand
    })
}
