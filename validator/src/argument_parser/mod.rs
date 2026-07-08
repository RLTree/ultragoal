use crate::command::{Args, Command};
use crate::{audit, cli};
use std::path::PathBuf;

mod authority;
mod help_request;
mod specialized;
#[cfg(test)]
mod tests;

pub(crate) fn parse_args_from(mut raw: Vec<String>) -> Result<Args, String> {
    let started = std::time::Instant::now();
    if raw.is_empty() {
        return Err(usage());
    }
    let mut root = authority::CliRoot::workspace_default();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--root" => {
                let value_index = i + 1;
                if value_index >= raw.len() {
                    return Err("missing value for --root".to_string());
                }
                root = authority::CliRoot::from_option_value(&raw[value_index])?;
                raw.drain(i..=i + 1);
            }
            _ => i += 1,
        }
    }
    if raw.is_empty() {
        return Err(usage());
    }
    let command = parse_command(&raw).map_err(|err| {
        let elapsed_ms = u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1);
        let _ = cli::audit::emit_parse_error_observability(root.as_path(), &raw, &err, elapsed_ms);
        err
    })?;
    Ok(Args {
        root: root.into_path_buf(),
        command,
    })
}

pub(crate) fn parse_command(raw: &[String]) -> Result<Command, String> {
    if help_request::is_help_request(raw) {
        return Ok(Command::Help);
    }
    Ok(match raw[0].as_str() {
        "help" | "--help" | "-h" => Command::Help,
        "audit" => parse_audit(&raw[1..])?,
        "source" if raw.get(1).map(String::as_str) == Some("audit") => parse_audit(&raw[2..])?,
        "target-repo" if raw.get(1).map(String::as_str) == Some("audit") => {
            parse_target_repo_audit(&raw[2..])?
        }
        "review-target" => {
            let args = strip_build_or_verify(&raw[1..]);
            let receipt = opt_path(args, "--receipt")?;
            Command::ReviewTarget {
                receipt,
                observability_receipt: cli::review::target::observability_receipt(args)?,
            }
        }
        "archive" => cli::archive::parse(&raw[1..])?,
        "review-round" => {
            let args = strip_build_or_verify(&raw[1..]);
            Command::ReviewRound {
                receipt: opt_path(args, "--receipt")?,
                validator_receipt: opt_path(args, "--validator-receipt")?,
                review_target_receipt: opt_path(args, "--review-target-receipt")?,
                archive_receipt: opt_path(args, "--archive-receipt")?,
                observability_receipt: cli::review::round::observability_receipt(args)?,
            }
        }
        "semantic-receipts" => Command::SemanticReceipts {
            input: opt_path(&raw[1..], "--input")?,
            out_dir: opt_path(&raw[1..], "--out-dir")?,
            implementation_kind: opt_string(&raw[1..], "--implementation-kind")
                .unwrap_or_else(|| "deterministic_backstop".to_string()),
            provider: opt_string(&raw[1..], "--provider"),
            model: opt_string(&raw[1..], "--model"),
            contract_id: opt_string(&raw[1..], "--contract-id")
                .unwrap_or_else(|| "ultragoal-semantic-classification".to_string()),
            contract_version: opt_string(&raw[1..], "--contract-version")
                .unwrap_or_else(|| "v1".to_string()),
            prompt_contract_digest: opt_string(&raw[1..], "--prompt-contract-digest"),
            producer_actor_id: opt_string(&raw[1..], "--producer-actor-id")
                .unwrap_or_else(|| "package-author".to_string()),
            classifier_actor_id: opt_string(&raw[1..], "--classifier-actor-id")
                .unwrap_or_else(|| "ultragoal-deterministic-backstop".to_string()),
        },
        "final-packet" | "packet" => {
            let Some(command) = cli::final_packet::parse(raw)? else {
                return Err(usage());
            };
            Command::FinalPacket(command)
        }
        "product" | "fit-repo" => {
            let Some(command) = cli::product::parse(raw)? else {
                return Err(usage());
            };
            Command::Product(command)
        }
        "standards-gardener" => {
            let Some(command) = cli::standards::parse(raw)? else {
                return Err(usage());
            };
            Command::Standards(command)
        }
        "package" if raw.get(1).map(String::as_str) == Some("digest") => Command::PackageDigest,
        "transaction" if raw.get(1).map(String::as_str) == Some("finalize") => {
            Command::TransactionalFinalization {
                receipt: opt_path(&raw[2..], "--receipt")?,
            }
        }
        _ => specialized::parse(raw)?,
    })
}

fn strip_build_or_verify(args: &[String]) -> &[String] {
    match args.first().map(String::as_str) {
        Some("build" | "verify") => &args[1..],
        _ => args,
    }
}

fn parse_audit(args: &[String]) -> Result<Command, String> {
    let mode = opt_string(args, "--mode").unwrap_or_else(|| "init".to_string());
    if !audit::receipt::speed::is_known_mode(&mode) {
        return Err(format!("invalid source audit --mode: {mode}"));
    }
    let target_repo =
        authority::optional_artifact_path(args, "--target-repo", "target repository root")?;
    Ok(Command::Audit {
        receipt: opt_path(args, "--receipt")?,
        red_report: authority::optional_artifact_path(args, "--red-report", "red fixture report")?
            .map(authority::CliArtifactPath::into_path_buf),
        target_repo: target_repo.map(authority::CliArtifactPath::into_path_buf),
        mode,
        require_observability: args.iter().any(|a| a == "--require-observability"),
        require_product_cohesion: args.iter().any(|a| a == "--require-product-cohesion"),
        jobs: opt_usize(args, "--jobs")?,
    })
}

fn parse_target_repo_audit(args: &[String]) -> Result<Command, String> {
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

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    authority::required_artifact_path(args, key, cli_option_role(key))
        .map(|path| path.into_path_buf())
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    authority::optional_text(args, key, cli_option_role(key))
        .ok()
        .flatten()
        .map(authority::CliText::into_string)
}

fn opt_usize(args: &[String], key: &str) -> Result<Option<usize>, String> {
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

fn cli_option_role(key: &str) -> &'static str {
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
