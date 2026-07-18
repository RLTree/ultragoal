#[cfg(not(test))]
use crate::Args;
#[cfg(test)]
use crate::{Args, Command};

#[cfg(not(test))]
pub fn run(args: Args) -> Result<i32, String> {
    crate::cli::successor_public::run_public(&args.root, args.outcome)
}

#[cfg(test)]
pub fn run(args: Args) -> Result<i32, String> {
    run_with_exit_code(args)
}

#[cfg(test)]
pub(crate) fn run_with_exit_code(args: Args) -> Result<i32, String> {
    let root = args.root;
    match args.command {
        Command::Audit {
            receipt,
            red_report,
            mode,
            require_observability,
            require_product_cohesion,
            jobs,
        } => crate::cli::audit::run(crate::cli::audit::RunArgs {
            root,
            receipt,
            red_report,
            mode,
            require_observability,
            require_product_cohesion,
            jobs,
        }),
        Command::Archive {
            zip,
            receipt,
            observability_receipt,
            zip_root,
            archive_purpose,
        } => crate::cli::archive::run(
            root,
            zip,
            receipt,
            observability_receipt,
            zip_root,
            archive_purpose,
        ),
        Command::FinalPacket(command) => crate::cli::final_packet::run(&root, &command),
        Command::Product(command) => crate::cli::product::run(&root, &command),
        Command::Standards(command) => crate::cli::standards::run(&root, &command),
        Command::Coverage(command) => crate::cli::coverage::run(&root, &command),
        Command::CurrentState(command) => crate::cli::current_state::run(&root, &command),
        Command::FoundationalTrace(command) => crate::cli::foundational_trace::run(&root, &command),
        Command::LineCaps(command) => crate::cli::line_caps::run(&root, &command),
        Command::LiveLoop(command) => crate::cli::live_loop::run(&root, &command),
        Command::ImpactedRustTests(command) => {
            crate::cli::live_loop::rust_tests::run(&root, &command)
        }
        Command::MandatoryLawValidation(command) => {
            crate::cli::mandatory_law_validation::run(&root, &command)
        }
        Command::Namespace(command) => crate::cli::namespace::run(&root, &command),
        Command::NextAction(command) => crate::cli::next_action::run(&root, &command),
        Command::Observe(command) => crate::cli::observe::run(&root, &command),
        Command::RedReport(command) => crate::cli::red_report::run(&root, &command),
        Command::SchemaValidation(command) => crate::cli::schema_validation::run(&root, &command),
        Command::Routine(command) => crate::cli::routine::run(&root, &command),
        Command::Session(command) => crate::cli::session::run(&root, &command),
        Command::SourceObligations(command) => crate::cli::source_obligations::run(&root, &command),
        Command::TypedBoundaries(command) => crate::cli::typed_boundaries::run(&root, &command),
        Command::PackageInventory(command) => crate::cli::package::inventory::run(&root, &command),
        Command::PackageDigest => crate::cli::package::digest::run(&root),
        Command::Successor(outcome) => crate::cli::successor_public::run_public(&root, outcome),
        Command::Help => {
            println!(
                "{}",
                crate::cli::successor::render_help(
                    crate::cli::successor::HelpTarget::Root,
                    crate::cli::successor::OutputMode::Human,
                )
            );
            Ok(0)
        }
    }
}
