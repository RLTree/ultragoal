use super::*;
use std::ffi::OsString;

pub(crate) fn parse_public_os_args_from(
    raw: Vec<OsString>,
) -> Result<Args, cli::successor::ParseFailure> {
    let parsed: cli::successor::ParsedCommandLine = cli::successor::parse_command_line(raw)?;
    let (root, outcome): (cli::successor::WorkspaceRoot, cli::successor::ParseOutcome) =
        parsed.into_parts();
    Ok(Args {
        root: root.into_path_buf(),
        #[cfg(not(test))]
        outcome,
        #[cfg(test)]
        command: Command::Successor(outcome),
    })
}

#[cfg(test)]
pub(crate) fn parse_public_args_from(raw: Vec<String>) -> Result<Args, String> {
    parse_public_os_args_from(raw.into_iter().map(OsString::from).collect())
        .map_err(cli::successor::ParseFailure::render)
}

#[cfg(test)]
pub(crate) fn parse_args_from(mut raw: Vec<String>) -> Result<Args, String> {
    let root = extract_root(&mut raw)?;
    if raw.is_empty() {
        return Err(usage());
    }
    let command = parse_command(&raw)?;
    Ok(Args {
        root: root.into_path_buf(),
        command,
    })
}

#[cfg(test)]
pub(super) fn extract_root(raw: &mut Vec<String>) -> Result<authority::CliRoot, String> {
    let mut root = authority::CliRoot::workspace_default();
    let output_mode = if raw.iter().any(|value| value == "--json") {
        cli::successor::OutputMode::Json
    } else {
        cli::successor::OutputMode::Human
    };
    let mut root_seen = false;
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--root" => {
                let value_index = i + 1;
                if root_seen {
                    return Err(root_parse_failure(
                        cli::successor::ParseErrorId::DuplicateOption,
                        output_mode,
                    ));
                }
                if value_index >= raw.len() || raw[value_index].starts_with('-') {
                    return Err(root_parse_failure(
                        cli::successor::ParseErrorId::MissingOptionValue,
                        output_mode,
                    ));
                }
                if raw[value_index].len() > cli::successor::MAX_ARGUMENT_BYTES {
                    return Err(root_parse_failure(
                        cli::successor::ParseErrorId::ArgumentTooLarge,
                        output_mode,
                    ));
                }
                root = authority::CliRoot::from_option_value(&raw[value_index]).map_err(|_| {
                    root_parse_failure(cli::successor::ParseErrorId::InvalidPath, output_mode)
                })?;
                root_seen = true;
                raw.drain(i..=i + 1);
            }
            _ => i += 1,
        }
    }
    Ok(root)
}

#[cfg(test)]
pub(crate) fn root_parse_failure(
    id: cli::successor::ParseErrorId,
    output_mode: cli::successor::OutputMode,
) -> String {
    cli::successor::ParseFailure::new(id, output_mode).render()
}

#[cfg(test)]
pub(crate) fn parse_command(raw: &[String]) -> Result<Command, String> {
    if successor_test_intent(raw) {
        return cli::successor_public::parse_public(raw).map(Command::Successor);
    }
    if help_request::is_help_request(raw) {
        return Ok(Command::Help);
    }
    Ok(match raw[0].as_str() {
        "help" | "--help" | "-h" => Command::Help,
        "audit" => parse_audit(&raw[1..])?,
        "source" if raw.get(1).map(String::as_str) == Some("audit") => parse_audit(&raw[2..])?,
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
        _ => specialized::parse(raw)?,
    })
}

#[cfg(test)]
pub(crate) fn successor_test_intent(raw: &[String]) -> bool {
    let positional = raw
        .iter()
        .filter(|token| token.as_str() != "--json")
        .take(2)
        .map(String::as_str)
        .collect::<Vec<_>>();
    let Some(head) = positional.first().copied() else {
        return false;
    };
    if matches!(head, "--help" | "-h" | "--version") {
        return true;
    }
    let Some(group) = cli::successor::Group::parse(head) else {
        return false;
    };
    match group {
        cli::successor::Group::Observe => {
            matches!(positional.get(1).copied(), Some("query" | "export"))
        }
        cli::successor::Group::Package => matches!(
            positional.get(1).copied(),
            Some("build" | "verify" | "install-test" | "publish")
        ),
        _ => true,
    }
}

#[cfg(test)]
pub(crate) fn strip_build_or_verify(args: &[String]) -> &[String] {
    match args.first().map(String::as_str) {
        Some("build" | "verify") => &args[1..],
        _ => args,
    }
}

#[cfg(test)]
pub(crate) fn parse_audit(args: &[String]) -> Result<Command, String> {
    let mode = opt_string(args, "--mode").unwrap_or_else(|| "init".to_string());
    if !audit::receipt::speed::is_known_mode(&mode) {
        return Err(format!("invalid source audit --mode: {mode}"));
    }
    Ok(Command::Audit {
        receipt: opt_path(args, "--receipt")?,
        red_report: authority::optional_artifact_path(args, "--red-report", "red fixture report")?
            .map(authority::CliArtifactPath::into_path_buf),
        mode,
        require_observability: args.iter().any(|a| a == "--require-observability"),
        require_product_cohesion: args.iter().any(|a| a == "--require-product-cohesion"),
        jobs: opt_usize(args, "--jobs")?,
    })
}
