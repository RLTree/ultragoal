use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

pub(crate) use types::{ProductCommand, ProductOperation};

pub(crate) fn parse(raw: &[String]) -> Result<Option<ProductCommand>, String> {
    match raw {
        [a, b, ..] if a == "product" && b == "prove-fitness" => Ok(Some(command(
            ProductOperation::ProductProveFitness,
            &raw[2..],
        )?)),
        [a, b, ..] if a == "fit-repo" && b == "prove" => {
            Ok(Some(command(ProductOperation::FitRepoProve, &raw[2..])?))
        }
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &ProductCommand) -> Result<i32, String> {
    let started = Instant::now();
    let outcome = run_minter(root, command);
    let status = outcome.status();
    let value = telemetry::emit(root, command, started, &outcome)?;
    if let Some(report) = outcome.report() {
        println!("{report}");
    }
    telemetry::print_summary(&value);
    Ok(i32::from(status != "pass"))
}

fn command(operation: ProductOperation, args: &[String]) -> Result<ProductCommand, String> {
    let observability_receipt = optional_path(args, "--observability-receipt")?
        .unwrap_or_else(|| PathBuf::from(operation.receipt_rel()));
    validate_root_relative(&observability_receipt, "product observability receipt")?;
    Ok(ProductCommand {
        operation,
        receipt_dir: opt_path(args, "--receipt-dir")?,
        observability_receipt,
    })
}

fn run_minter(root: &Path, command: &ProductCommand) -> telemetry::ProductOutcome {
    let out_dir = match output_dir(root, &command.receipt_dir) {
        Ok(path) => path,
        Err(err) => return telemetry::ProductOutcome::Failure(err),
    };
    match receipts::mint_all(root, &command.receipt_dir, &out_dir) {
        Ok(report) => telemetry::ProductOutcome::Report(report),
        Err(err) => telemetry::ProductOutcome::Failure(err),
    }
}

fn output_dir(root: &Path, dir: &Path) -> Result<PathBuf, String> {
    validate_root_relative(dir, "product receipt directory")?;
    Ok(root.join(dir))
}

fn validate_root_relative(dir: &Path, label: &str) -> Result<(), String> {
    if dir.is_absolute() {
        return Err(format!("{label} must be root-relative"));
    }
    if dir
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{}: {label} escapes package root", dir.display()));
    }
    Ok(())
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    optional_path(args, key)?.ok_or_else(|| format!("missing required argument {key}"))
}

fn optional_path(args: &[String], key: &str) -> Result<Option<PathBuf>, String> {
    args.iter()
        .position(|arg| arg == key)
        .map(|index| {
            args.get(index + 1)
                .map(PathBuf::from)
                .ok_or_else(|| format!("missing required argument {key}"))
        })
        .transpose()
}

pub(crate) fn receipt_row(path: &str, digest: &str, status: &str) -> Value {
    json!({"path": path, "digest": digest, "status": status})
}

pub(crate) mod receipts;
mod telemetry;
mod types;
