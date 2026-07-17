use crate::cli;
use crate::command::Command;

pub(super) fn parse(raw: &[String]) -> Result<Command, String> {
    if let Some(command) = cli::coverage::parse(raw)? {
        Ok(Command::Coverage(command))
    } else if let Some(command) = cli::package::inventory::parse(raw)? {
        Ok(Command::PackageInventory(command))
    } else if let Some(command) = cli::line_caps::parse(raw)? {
        Ok(Command::LineCaps(command))
    } else if let Some(command) = cli::live_loop::rust_tests::parse(raw)? {
        Ok(Command::ImpactedRustTests(command))
    } else if let Some(command) = cli::live_loop::parse(raw)? {
        Ok(Command::LiveLoop(command))
    } else if let Some(command) = cli::mandatory_law_validation::parse(raw)? {
        Ok(Command::MandatoryLawValidation(command))
    } else if let Some(command) = cli::namespace::parse(raw)? {
        Ok(Command::Namespace(command))
    } else if let Some(command) = cli::next_action::parse(raw)? {
        Ok(Command::NextAction(command))
    } else if let Some(command) = cli::observe::parse(raw)? {
        Ok(Command::Observe(command))
    } else if let Some(command) = crate::red::fixture::scheduler::parse(raw)? {
        Ok(Command::FixtureSchedule(command))
    } else if let Some(command) = cli::red_report::parse(raw)? {
        Ok(Command::RedReport(command))
    } else if let Some(command) = cli::schema_validation::parse(raw)? {
        Ok(Command::SchemaValidation(command))
    } else if let Some(command) = cli::current_state::parse(raw)? {
        Ok(Command::CurrentState(command))
    } else if let Some(command) = cli::routine::parse(raw)? {
        Ok(Command::Routine(command))
    } else if let Some(command) = cli::session::parse(raw)? {
        Ok(Command::Session(command))
    } else if let Some(command) = cli::source_obligations::parse(raw)? {
        Ok(Command::SourceObligations(command))
    } else if let Some(command) = cli::typed_boundaries::parse(raw)? {
        Ok(Command::TypedBoundaries(command))
    } else if let Some(command) = cli::foundational_trace::parse(raw)? {
        Ok(Command::FoundationalTrace(command))
    } else {
        Err(super::usage())
    }
}
