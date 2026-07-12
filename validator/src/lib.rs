mod agent_manifest;
mod agent_roles;
mod api_witness;
mod archive;
mod argument_parser;
mod audit;
mod claim;
mod claim_semantics;
mod claims;
mod cli;
pub use cli::capture;
mod command;
mod command_run;
mod command_witness;
pub mod context;
mod contract_check_ids;
mod digest;
pub mod distribution;
pub mod evaluation;
mod fixture_scheduler;
pub mod inventory;
mod json_boundary;
pub mod migration;
pub mod observability;
pub mod orchestration;
mod output_path;
mod package;
mod plugin_manifest;
pub mod plugin_product;
mod red;
pub mod repository_fit;
mod review;
pub mod routine_work;
mod scheduler;
mod schema_catalog;
#[cfg(test)]
pub(crate) mod self_tests;
mod semantic;
mod skill_links;
pub mod state;
mod target_fixtures;
mod target_repo;
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
    argument_parser::parse_public_args_from(std::env::args().skip(1).collect())
        .and_then(command_run::run)
        .unwrap_or_else(|err| {
            eprintln!("{err}");
            2
        })
}
