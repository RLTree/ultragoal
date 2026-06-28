use crate::{Args, Command};

pub fn run(args: Args) -> Result<i32, String> {
    run_with_exit_code(args)
}

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
        } => run_audit(
            root,
            receipt,
            red_report,
            target_repo,
            mode,
            require_observability,
            require_product_cohesion,
        ),
        Command::ReviewTarget { receipt } => run_review_target(root, receipt),
        Command::Archive {
            zip,
            receipt,
            zip_root,
            archive_purpose,
        } => run_archive(root, zip, receipt, zip_root, archive_purpose),
        Command::ReviewRound {
            receipt,
            validator_receipt,
            review_target_receipt,
            archive_receipt,
        } => run_review_round(
            root,
            receipt,
            validator_receipt,
            review_target_receipt,
            archive_receipt,
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
        Command::TransactionalFinalization { receipt } => {
            run_transactional_finalization(root, receipt)
        }
        Command::Control(command) => crate::cli::control::plane::run(&root, &command),
        Command::Performance(command) => crate::cli::performance::run(&root, &command),
        Command::Rust(command) => crate::cli::rust::run(&root, &command),
        Command::Garbage(command) => crate::cli::garbage::collection::run(&root, &command),
        Command::PackageDigest => {
            println!("{}", crate::package::inventory::package_digest(&root)?);
            Ok(0)
        }
    }
}

fn run_transactional_finalization(
    root: std::path::PathBuf,
    receipt: std::path::PathBuf,
) -> Result<i32, String> {
    let value = crate::cli::control::plane::transactional::receipt(&root)?;
    crate::json_boundary::write_json(&receipt, &value)?;
    println!(
        "ultragoal-transaction {} receipt={}",
        value["status"],
        receipt.display()
    );
    Ok(i32::from(
        value.get("status").and_then(serde_json::Value::as_str) != Some("pass"),
    ))
}

fn run_audit(
    root: std::path::PathBuf,
    receipt: std::path::PathBuf,
    red_report: Option<std::path::PathBuf>,
    target_repo: Option<std::path::PathBuf>,
    mode: String,
    require_observability: bool,
    require_product_cohesion: bool,
) -> Result<i32, String> {
    let command_text = command_text(&root);
    crate::audit::run(crate::audit::AuditOptions {
        root,
        receipt,
        red_report,
        target_repo,
        mode,
        require_observability,
        require_product_cohesion,
        command_text,
    })
}

fn command_text(root: &std::path::Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}

fn run_review_target(root: std::path::PathBuf, receipt: std::path::PathBuf) -> Result<i32, String> {
    let receipt_value = crate::package::build_review_target_receipt(&root)?;
    crate::json_boundary::write_json(&receipt, &receipt_value)?;
    println!(
        "review-target pass digest={} receipt={}",
        receipt_value["review_target_digest"]
            .as_str()
            .unwrap_or("<missing>"),
        receipt.display()
    );
    Ok(0)
}

fn run_archive(
    root: std::path::PathBuf,
    zip: std::path::PathBuf,
    receipt: std::path::PathBuf,
    zip_root: String,
    archive_purpose: String,
) -> Result<i32, String> {
    let receipt_value = crate::archive::build_archive(&root, &zip, &zip_root, &archive_purpose)?;
    crate::json_boundary::write_json(&receipt, &receipt_value)?;
    println!(
        "archive pass digest={} receipt={}",
        receipt_value["archive"]["digest"]
            .as_str()
            .unwrap_or("<missing>"),
        receipt.display()
    );
    Ok(0)
}

fn run_review_round(
    root: std::path::PathBuf,
    receipt: std::path::PathBuf,
    validator_receipt: std::path::PathBuf,
    review_target_receipt: std::path::PathBuf,
    archive_receipt: std::path::PathBuf,
) -> Result<i32, String> {
    let anchors = crate::review::round::AnchorPaths {
        validator_receipt,
        review_target_receipt,
        archive_receipt,
    };
    match crate::review::round::validate_files(&root, &receipt, &anchors) {
        Ok(()) => {
            println!("review-round pass receipt={}", receipt.display());
            Ok(0)
        }
        Err(err) => {
            println!("review-round fail receipt={}: {err}", receipt.display());
            Ok(1)
        }
    }
}

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
