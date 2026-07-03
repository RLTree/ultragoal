use crate::command::{Args, Command};
use crate::{audit, cli};
use std::path::PathBuf;

pub(crate) fn parse_args_from(mut raw: Vec<String>) -> Result<Args, String> {
    let started = std::time::Instant::now();
    if raw.is_empty() {
        return Err(usage());
    }
    let mut root = PathBuf::from(".");
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--root" => {
                let value_index = i + 1;
                if value_index >= raw.len() {
                    return Err("missing value for --root".to_string());
                }
                root = PathBuf::from(raw[value_index].clone());
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
        let _ = cli::audit::emit_parse_error_observability(&root, &raw, &err, elapsed_ms);
        err
    })?;
    Ok(Args { root, command })
}

pub(crate) fn parse_command(raw: &[String]) -> Result<Command, String> {
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
        "package-digest" => Command::PackageDigest,
        "transaction" if raw.get(1).map(String::as_str) == Some("finalize") => {
            Command::TransactionalFinalization {
                receipt: opt_path(&raw[2..], "--receipt")?,
            }
        }
        _ => parse_specialized_command(raw)?,
    })
}

fn parse_specialized_command(raw: &[String]) -> Result<Command, String> {
    if let Some(command) = cli::coverage::parse(raw)? {
        Ok(Command::Coverage(command))
    } else if let Some(command) = cli::package::inventory::parse(raw)? {
        Ok(Command::PackageInventory(command))
    } else if let Some(command) = cli::performance::parse(raw)? {
        Ok(Command::Performance(command))
    } else if let Some(command) = cli::rust::parse(raw)? {
        Ok(Command::Rust(command))
    } else if let Some(command) = cli::garbage::collection::parse(raw)? {
        Ok(Command::Garbage(command))
    } else if let Some(command) = cli::halo::parse(raw)? {
        Ok(Command::Halo(command))
    } else if let Some(command) = cli::improvement_loop::parse(raw)? {
        Ok(Command::ImprovementLoop(command))
    } else if let Some(command) = cli::line_caps::parse(raw)? {
        Ok(Command::LineCaps(command))
    } else if let Some(command) = cli::live_loop::parse(raw)? {
        Ok(Command::LiveLoop(command))
    } else if let Some(command) = cli::mandatory_law_validation::parse(raw)? {
        Ok(Command::MandatoryLawValidation(command))
    } else if let Some(command) = cli::namespace::parse(raw)? {
        Ok(Command::Namespace(command))
    } else if let Some(command) = cli::observe::parse(raw)? {
        Ok(Command::Observe(command))
    } else if let Some(command) = cli::openai::parse(raw)? {
        Ok(Command::OpenAi(command))
    } else if let Some(command) = cli::promptfoo::parse(raw)? {
        Ok(Command::Promptfoo(command))
    } else if let Some(command) = cli::red_report::parse(raw)? {
        Ok(Command::RedReport(command))
    } else if let Some(command) = cli::schema_validation::parse(raw)? {
        Ok(Command::SchemaValidation(command))
    } else if let Some(command) = cli::current_state::parse(raw)? {
        Ok(Command::CurrentState(command))
    } else if let Some(command) = cli::routine::parse(raw)? {
        Ok(Command::Routine(command))
    } else if let Some(command) = cli::session::parse(raw)? {
        Ok(Command::Session(command))
    } else if let Some(command) = cli::source_obligations::parse(raw)? {
        Ok(Command::SourceObligations(command))
    } else if let Some(command) = cli::typed_boundaries::parse(raw)? {
        Ok(Command::TypedBoundaries(command))
    } else if let Some(command) = cli::foundational_trace::parse(raw)? {
        Ok(Command::FoundationalTrace(command))
    } else if let Some(command) = cli::control::plane::parse(raw) {
        Ok(Command::Control(command))
    } else {
        Err(usage())
    }
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
    Ok(Command::Audit {
        receipt: opt_path(args, "--receipt")?,
        red_report: opt_string(args, "--red-report").map(PathBuf::from),
        target_repo: opt_string(args, "--target-repo").map(PathBuf::from),
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
    opt_string(args, key)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1].clone())
}

fn opt_usize(args: &[String], key: &str) -> Result<Option<usize>, String> {
    let Some(raw) = opt_string(args, key) else {
        return Ok(None);
    };
    raw.parse::<usize>()
        .map(Some)
        .map_err(|_| format!("invalid numeric value for {key}: {raw}"))
}

pub(crate) fn usage() -> String {
    cli::usage::text().to_string()
}
