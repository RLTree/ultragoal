use std::path::{Path, PathBuf};

#[cfg(test)]
mod edge_tests;
mod observability;
#[cfg(test)]
mod tests;

pub(crate) struct RunArgs {
    pub(crate) root: PathBuf,
    pub(crate) receipt: PathBuf,
    pub(crate) red_report: Option<PathBuf>,
    pub(crate) target_repo: Option<PathBuf>,
    pub(crate) mode: String,
    pub(crate) require_observability: bool,
    pub(crate) require_product_cohesion: bool,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn run(args: RunArgs) -> Result<i32, String> {
    let started = std::time::Instant::now();
    let root = args.root.clone();
    let receipt = args.receipt.clone();
    let target_repo = args.target_repo.clone();
    let red_report = args.red_report.clone();
    let options = crate::audit::AuditOptions {
        root: args.root,
        receipt: args.receipt,
        red_report: args.red_report,
        target_repo: args.target_repo,
        mode: args.mode,
        require_observability: args.require_observability,
        require_product_cohesion: args.require_product_cohesion,
        jobs: args.jobs,
        command_text: command_text(&root),
    };
    let result = crate::audit::run(options);
    match result {
        Ok(code) => {
            observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                code,
                target_repo.is_some(),
                None,
                observability::RuntimeFacts::from_elapsed_ms(elapsed_ms(started)),
            )?;
            Ok(code)
        }
        Err(err) => {
            let _ = observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                1,
                target_repo.is_some(),
                Some(&err),
                observability::RuntimeFacts::from_elapsed_ms(elapsed_ms(started)),
            );
            Err(err)
        }
    }
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn command_text(root: &Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}
