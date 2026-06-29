use crate::cli::performance::types::{BudgetClass, PerformanceOperation};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) use proof::receipt;

#[derive(Debug)]
pub(crate) struct PerformanceCommand {
    pub(crate) operation: PerformanceOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) class: BudgetClass,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<PerformanceCommand>, String> {
    let operation = match raw {
        [a, b, ..] if a == "performance" && b == "prove" => PerformanceOperation::Prove,
        [a, b, ..] if a == "performance" && b == "verify" => PerformanceOperation::Verify,
        [a, b, ..] if a == "performance" && b == "budgets" => PerformanceOperation::Budgets,
        [a, b, c, ..] if a == "self" && b == "performance" && c == "prove" => {
            PerformanceOperation::SelfProve
        }
        _ => return Ok(None),
    };
    let class = match opt_string(raw, "--class") {
        Some(raw) => BudgetClass::from_str(&raw)
            .ok_or_else(|| "invalid --class performance budget".to_string())?,
        None => default_class(operation),
    };
    Ok(Some(PerformanceCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
        class,
    }))
}

pub(crate) fn run(root: &Path, command: &PerformanceCommand) -> Result<i32, String> {
    let start = Instant::now();
    let mut receipt = receipt(root, command, 0)?;
    proof::apply_status(
        &mut receipt,
        command.class,
        start.elapsed().as_millis() as u64,
    );
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        println!(
            "ultragoal-performance {} operation={} class={} receipt={}",
            receipt["status"],
            command.operation.id(),
            command.class.id(),
            path.display()
        );
    } else {
        println!("{receipt}");
    }
    Ok(receipt["exit_code"].as_i64().unwrap_or(1) as i32)
}

fn default_class(operation: PerformanceOperation) -> BudgetClass {
    match operation {
        PerformanceOperation::Budgets => BudgetClass::Instant,
        PerformanceOperation::Verify => BudgetClass::Focused,
        PerformanceOperation::Prove | PerformanceOperation::SelfProve => BudgetClass::StrictLocal,
    }
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}
pub(crate) mod proof;
pub(crate) mod receipt;
pub(crate) mod types;
