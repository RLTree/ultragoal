use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct StandardsCommand {
    pub(crate) receipt: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<StandardsCommand>, String> {
    match raw {
        [a, b, ..] if a == "standards-gardener" && b == "rebind" => Ok(Some(StandardsCommand {
            receipt: opt_path(raw, "--receipt")?,
        })),
        [a] if a == "standards-gardener" => Ok(None),
        [a, ..] if a == "standards-gardener" => Err("unknown standards-gardener command".into()),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &StandardsCommand) -> Result<i32, String> {
    let value = gardener::rebind(root, &command.receipt)?;
    println!(
        "ultragoal-standards-gardener {} receipt={}",
        value["status"],
        command.receipt.display()
    );
    Ok(0)
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

pub(crate) mod gardener;
