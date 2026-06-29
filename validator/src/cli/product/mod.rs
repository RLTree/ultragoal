use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct ProductCommand {
    pub(crate) receipt_dir: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<ProductCommand>, String> {
    match raw {
        [a, b, ..] if a == "product" && b == "prove-fitness" => Ok(Some(ProductCommand {
            receipt_dir: opt_path(&raw[2..], "--receipt-dir")?,
        })),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &ProductCommand) -> Result<i32, String> {
    let out_dir = output_dir(root, &command.receipt_dir)?;
    let report = receipts::mint_all(root, &command.receipt_dir, &out_dir)?;
    println!("{report}");
    Ok(i32::from(
        report.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

fn output_dir(root: &Path, dir: &Path) -> Result<PathBuf, String> {
    if dir.is_absolute() {
        return Err("product receipt directory must be root-relative".to_string());
    }
    if dir
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!(
            "{}: product receipt directory escapes package root",
            dir.display()
        ));
    }
    Ok(root.join(dir))
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

pub(crate) fn receipt_row(path: &str, digest: &str, status: &str) -> Value {
    json!({"path": path, "digest": digest, "status": status})
}

pub(crate) mod receipts;
