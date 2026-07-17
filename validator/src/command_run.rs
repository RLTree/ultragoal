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
            target_repo,
            mode,
            require_observability,
            require_product_cohesion,
            jobs,
        } => crate::cli::audit::run(crate::cli::audit::RunArgs {
            root,
            receipt,
            red_report,
            target_repo,
            mode,
            require_observability,
            require_product_cohesion,
            jobs,
        }),
        Command::ReviewTarget {
            receipt,
            observability_receipt,
        } => crate::cli::review::target::run(root, receipt, observability_receipt),
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
        Command::ReviewRound {
            receipt,
            validator_receipt,
            review_target_receipt,
            archive_receipt,
            observability_receipt,
        } => crate::cli::review::round::run(
            root,
            receipt,
            validator_receipt,
            review_target_receipt,
            archive_receipt,
            observability_receipt,
        ),
        Command::SemanticReceipts {
            input,
            out_dir,
            implementation_kind,
            provider,
            model,
            contract_id,
            contract_version,
            prompt_contract_digest,
            producer_actor_id,
            classifier_actor_id,
        } => run_semantic_receipts(SemanticReceiptArgs {
            root,
            input,
            out_dir,
            implementation_kind,
            provider,
            model,
            contract_id,
            contract_version,
            prompt_contract_digest,
            producer_actor_id,
            classifier_actor_id,
        }),
        Command::FinalPacket(command) => crate::cli::final_packet::run(&root, &command),
        Command::Product(command) => crate::cli::product::run(&root, &command),
        Command::Standards(command) => crate::cli::standards::run(&root, &command),
        Command::TransactionalFinalization { receipt } => {
            run_transactional_finalization(root, receipt)
        }
        Command::Control(command) => crate::cli::control::plane::run(&root, &command),
        Command::Coverage(command) => crate::cli::coverage::run(&root, &command),
        Command::CurrentState(command) => crate::cli::current_state::run(&root, &command),
        Command::FoundationalTrace(command) => crate::cli::foundational_trace::run(&root, &command),
        Command::Performance(command) => crate::cli::performance::run(&root, &command),
        Command::Rust(command) => crate::cli::rust::run(&root, &command),
        Command::Garbage(command) => crate::cli::garbage::collection::run(&root, &command),
        Command::Halo(command) => crate::cli::halo::run(&root, &command),
        Command::ImprovementLoop(command) => crate::cli::improvement_loop::run(&root, &command),
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
        Command::OpenAi(command) => crate::cli::openai::run(&root, &command),
        Command::Promptfoo(command) => crate::cli::promptfoo::run(&root, &command),
        Command::RedReport(command) => crate::cli::red_report::run(&root, &command),
        Command::FixtureSchedule(command) => crate::red::fixture::scheduler::run(&root, &command),
        Command::SchemaValidation(command) => crate::cli::schema_validation::run(&root, &command),
        Command::Routine(command) => crate::cli::routine::run(&root, &command),
        Command::Session(command) => crate::cli::session::run(&root, &command),
        Command::SourceObligations(command) => crate::cli::source_obligations::run(&root, &command),
        Command::TypedBoundaries(command) => crate::cli::typed_boundaries::run(&root, &command),
        Command::PackageInventory(command) => crate::cli::package::inventory::run(&root, &command),
        Command::PackageDigest => crate::cli::package::digest::run(&root),
        Command::Successor(outcome) => crate::cli::successor_public::run_public(&root, outcome),
        Command::Help => {
            println!("{}", crate::cli::usage::text());
            Ok(0)
        }
    }
}

#[cfg(test)]
fn run_transactional_finalization(
    root: std::path::PathBuf,
    receipt: std::path::PathBuf,
) -> Result<i32, String> {
    let started = std::time::Instant::now();
    let claim_receipt = crate::output_path::claim_artifact_path(
        &root,
        &receipt,
        "transactional finalization receipt",
    )?;
    let mut value = crate::cli::control::plane::transactional::receipt(&root)?;
    crate::cli::control::plane::transactional::telemetry::attach(
        &root,
        &claim_receipt,
        &mut value,
        started,
    )?;
    crate::json_boundary::write_json(&claim_receipt, &value)?;
    crate::cli::control::plane::transactional::stdout::print(&root, &claim_receipt, &value);
    Ok(i32::from(
        value.get("status").and_then(serde_json::Value::as_str) != Some("pass"),
    ))
}

#[cfg(test)]
struct SemanticReceiptArgs {
    root: std::path::PathBuf,
    input: std::path::PathBuf,
    out_dir: std::path::PathBuf,
    implementation_kind: String,
    provider: Option<String>,
    model: Option<String>,
    contract_id: String,
    contract_version: String,
    prompt_contract_digest: Option<String>,
    producer_actor_id: String,
    classifier_actor_id: String,
}

#[cfg(test)]
fn run_semantic_receipts(args: SemanticReceiptArgs) -> Result<i32, String> {
    crate::semantic::receipt::generate(crate::semantic::receipt::GenerateOptions {
        root: args.root,
        input: args.input,
        out_dir: args.out_dir,
        implementation_kind: args.implementation_kind,
        provider: args.provider,
        model: args.model,
        contract_id: args.contract_id,
        contract_version: args.contract_version,
        prompt_contract_digest: args.prompt_contract_digest,
        producer_actor_id: args.producer_actor_id,
        classifier_actor_id: args.classifier_actor_id,
    })?;
    Ok(0)
}
