macro_rules! include_production_package_module {
    () => {
        mod product;
    };
}

mod agent_manifest;
mod agent_roles;
mod api_witness;
#[cfg(test)]
mod archive;
mod argument_parser;
mod audit;
mod claim_semantics;
mod cli;
pub use cli::capture;
#[cfg(test)]
mod command;
mod command_run;
mod command_witness;
pub mod context;
mod contract_check_ids;
mod digest;
pub mod distribution;
#[cfg(test)]
pub mod evaluation;
#[cfg(test)]
mod fixture_scheduler;
mod generated_authority;
pub mod inventory;
mod json_boundary;
#[cfg(test)]
pub mod migration;
pub mod observability;
pub mod orchestration;
mod output_path;
mod package;
mod plugin_manifest;
pub mod plugin_product;
mod red;
pub mod repository_fit;
pub mod routine_work;
mod scheduler;
mod schema_catalog;
#[cfg(test)]
pub(crate) mod self_tests;
mod skill_links;
pub mod state;
#[cfg(not(test))]
pub(crate) struct Args {
    pub(crate) root: std::path::PathBuf,
    pub(crate) outcome: cli::successor::ParseOutcome,
}

#[cfg(test)]
pub(crate) use command::{Args, Command};

#[cfg(test)]
pub(crate) fn parse_args_from(raw: Vec<String>) -> Result<Args, String> {
    argument_parser::parse_args_from(raw)
}

#[cfg(test)]
pub(crate) fn parse_command(raw: &[String]) -> Result<Command, String> {
    argument_parser::parse_command(raw)
}

#[cfg(test)]
pub(crate) fn usage() -> String {
    argument_parser::usage()
}

pub fn main_entry() -> i32 {
    let args =
        match argument_parser::parse_public_os_args_from(std::env::args_os().skip(1).collect()) {
            Ok(args) => args,
            Err(failure) => {
                let exit = failure.error.exit_class().code();
                eprintln!("{}", failure.render());
                return exit;
            }
        };
    command_run::run(args).unwrap_or_else(|err| {
        eprintln!("{err}");
        2
    })
}
