use super::*;

pub(crate) fn parse_target_repo_audit(args: &[String]) -> Result<Command, String> {
    let mode = opt_string(args, "--mode").unwrap_or_else(|| "init".to_string());
    if !audit::receipt::speed::is_known_mode(&mode) {
        return Err(format!("invalid target-repo audit --mode: {mode}"));
    }
    Ok(Command::Audit {
        receipt: opt_path(args, "--receipt")?,
        red_report: None,
        target_repo: Some(opt_path(args, "--surface-root")?),
        mode,
        require_observability: args.iter().any(|a| a == "--require-observability"),
        require_product_cohesion: args.iter().any(|a| a == "--require-product-cohesion"),
        jobs: opt_usize(args, "--jobs")?,
    })
}

pub(crate) fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    authority::required_artifact_path(args, key, cli_option_role(key))
        .map(|path| path.into_path_buf())
}

pub(crate) fn opt_string(args: &[String], key: &str) -> Option<String> {
    authority::optional_text(args, key, cli_option_role(key))
        .ok()
        .flatten()
        .map(authority::CliText::into_string)
}

pub(crate) fn opt_usize(args: &[String], key: &str) -> Result<Option<usize>, String> {
    let Some(raw) = authority::option_value(args, key) else {
        return Ok(None);
    };
    raw.parse::<usize>()
        .map(Some)
        .map_err(|_| format!("invalid numeric value for {key}: {raw}"))
}

pub(crate) fn usage() -> String {
    cli::usage::text().to_string()
}

pub(crate) fn cli_option_role(key: &str) -> &'static str {
    match key {
        "--archive-receipt" => "archive receipt path",
        "--archive-purpose" => "archive purpose",
        "--cache-mode" => "verified cache mode",
        "--class" => "performance budget class",
        "--classifier-actor-id" => "semantic receipt classifier actor",
        "--contract-id" => "semantic receipt contract id",
        "--contract-version" => "semantic receipt contract version",
        "--implementation-kind" => "semantic receipt implementation kind",
        "--input" => "semantic receipt input path",
        "--mode" => "source audit mode",
        "--model" => "external model identifier",
        "--out-dir" => "semantic receipt output directory",
        "--producer-actor-id" => "semantic receipt producer actor",
        "--prompt-contract-digest" => "semantic prompt contract digest",
        "--provider" => "external provider identifier",
        "--receipt" => "claim receipt path",
        "--red-report" => "red fixture report path",
        "--review-target-receipt" => "review target receipt path",
        "--surface-root" => "target repository surface root",
        "--target-repo" => "target repository root",
        "--validator-receipt" => "validator receipt path",
        "--zip" => "archive zip path",
        "--zip-root" => "archive zip root",
        _ => panic!("unregistered CLI option product role: {key}"),
    }
}
