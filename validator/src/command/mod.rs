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
    },
    Archive {
        zip: PathBuf,
        receipt: PathBuf,
        zip_root: String,
        archive_purpose: String,
    },
    ReviewRound {
        receipt: PathBuf,
        validator_receipt: PathBuf,
        review_target_receipt: PathBuf,
        archive_receipt: PathBuf,
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
    FoundationalTrace(crate::cli::foundational_trace::FoundationalTraceCommand),
    Performance(crate::cli::performance::PerformanceCommand),
    Rust(crate::cli::rust::RustCommand),
    Garbage(crate::cli::garbage::collection::GarbageCommand),
    Halo(crate::cli::halo::HaloCommand),
    ImprovementLoop(crate::cli::improvement_loop::ImprovementLoopCommand),
    MandatoryLawValidation(crate::cli::mandatory_law_validation::MandatoryLawValidationCommand),
    Observe(crate::cli::observe::types::ObserveCommand),
    OpenAi(crate::cli::openai::OpenAiCommand),
    Promptfoo(crate::cli::promptfoo::PromptfooCommand),
    RedReport(crate::cli::red_report::RedReportCommand),
    SchemaValidation(crate::cli::schema_validation::SchemaValidationCommand),
    Routine(crate::cli::routine::RoutineCommand),
    Session(crate::cli::session::SessionCommand),
    SourceObligations(crate::cli::source_obligations::SourceObligationsCommand),
    PackageDigest,
    Help,
}
