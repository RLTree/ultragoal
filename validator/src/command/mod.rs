use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) root: PathBuf,
    pub(crate) command: Command,
}

#[derive(Debug)]
pub(crate) enum Command {
    Audit {
        receipt: PathBuf,
        red_report: Option<PathBuf>,
        target_repo: Option<PathBuf>,
        mode: String,
        require_observability: bool,
        require_product_cohesion: bool,
        jobs: Option<usize>,
    },
    ReviewTarget {
        receipt: PathBuf,
        observability_receipt: PathBuf,
    },
    Archive {
        zip: PathBuf,
        receipt: PathBuf,
        observability_receipt: PathBuf,
        zip_root: String,
        archive_purpose: String,
    },
    ReviewRound {
        receipt: PathBuf,
        validator_receipt: PathBuf,
        review_target_receipt: PathBuf,
        archive_receipt: PathBuf,
        observability_receipt: PathBuf,
    },
    SemanticReceipts {
        input: PathBuf,
        out_dir: PathBuf,
        implementation_kind: String,
        provider: Option<String>,
        model: Option<String>,
        contract_id: String,
        contract_version: String,
        prompt_contract_digest: Option<String>,
        producer_actor_id: String,
        classifier_actor_id: String,
    },
    FinalPacket(crate::cli::final_packet::FinalPacketCommand),
    Product(crate::cli::product::ProductCommand),
    Standards(crate::cli::standards::StandardsCommand),
    TransactionalFinalization {
        receipt: PathBuf,
    },
    Control(crate::cli::control::plane::ControlCommand),
    Coverage(crate::cli::coverage::CoverageCommand),
    CurrentState(crate::cli::current_state::CurrentStateCommand),
    FoundationalTrace(crate::cli::foundational_trace::FoundationalTraceCommand),
    LineCaps(crate::cli::line_caps::LineCapsCommand),
    LiveLoop(crate::cli::live_loop::LiveLoopCommand),
    ImpactedRustTests(crate::cli::live_loop::rust_tests::ImpactedRustTestsCommand),
    MandatoryLawValidation(crate::cli::mandatory_law_validation::MandatoryLawValidationCommand),
    Namespace(crate::cli::namespace::NamespaceCommand),
    NextAction(crate::cli::next_action::NextActionCommand),
    Observe(crate::cli::observe::command::ObserveCommand),
    RedReport(crate::cli::red_report::RedReportCommand),
    FixtureSchedule(crate::red::fixture::scheduler::FixtureSchedulerCommand),
    SchemaValidation(crate::cli::schema_validation::SchemaValidationCommand),
    Routine(crate::cli::routine::RoutineCommand),
    Session(crate::cli::session::SessionCommand),
    SourceObligations(crate::cli::source_obligations::SourceObligationsCommand),
    TypedBoundaries(crate::cli::typed_boundaries::TypedBoundariesCommand),
    PackageInventory(crate::cli::package::inventory::PackageInventoryCommand),
    PackageDigest,
    Successor(crate::cli::successor::ParseOutcome),
    Help,
}
