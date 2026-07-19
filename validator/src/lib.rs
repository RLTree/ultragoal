macro_rules! include_production_package_module {
    () => {
        mod product;
    };
}

mod agent_manifest;
mod agent_roles;
mod api_witness;
mod argument_parser;
mod audit;
mod cli;
pub use cli::capture;
mod command_run;
mod command_witness;
pub mod context;
mod contract_amendment;
mod contract_check_ids;
mod digest;
pub mod distribution;
pub mod evaluation;
pub mod fixture_scheduler;
#[cfg(test)]
#[path = "fixture_scheduler/tests/mod.rs"]
mod fixture_scheduler_tests;
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
pub(crate) struct Args {
    pub(crate) root: std::path::PathBuf,
    pub(crate) outcome: cli::successor::ParseOutcome,
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
