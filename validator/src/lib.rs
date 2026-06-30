mod archive;
mod audit;
mod claim;
mod claim_semantics;
mod cli;
mod command;
mod command_run;
mod contract_check_ids;
mod digest;
mod json_boundary;
mod output_path;
mod package;
mod red;
mod review;
mod schema_catalog;
#[cfg(test)]
pub(crate) mod self_tests;
mod semantic;
mod skill_links;
mod target_fixtures;
mod target_repo;
pub(crate) use command::{Args, Command};
use std::path::PathBuf;

pub fn main_entry() -> i32 {
    match parse_args().and_then(command_run::run) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            2
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut raw = std::env::args().skip(1).collect::<Vec<_>>();
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
                let value = raw[value_index].clone();
                root = PathBuf::from(value);
                raw.drain(i..=i + 1);
            }
            _ => i += 1,
        }
    }
    if raw.is_empty() {
        return Err(usage());
    }
    let parsed_command = parse_command(&raw);
    if parsed_command.is_err() {
        return Err(parsed_command.err().expect("checked parser error"));
    }
    let command = parsed_command.ok().expect("checked parsed command");
    Ok(Args { root, command })
}

fn parse_command(raw: &[String]) -> Result<Command, String> {
    Ok(match raw[0].as_str() {
        "audit" => parse_audit(&raw[1..])?,
        "source" if raw.get(1).map(String::as_str) == Some("audit") => parse_audit(&raw[2..])?,
        "review-target" => {
            let args = strip_build_or_verify(&raw[1..]);
            let receipt = opt_path(args, "--receipt")?;
            Command::ReviewTarget { receipt }
        }
        "archive" => Command::Archive {
            zip: opt_path(strip_build_or_verify(&raw[1..]), "--zip")?,
            receipt: opt_path(strip_build_or_verify(&raw[1..]), "--receipt")?,
            zip_root: opt_string(strip_build_or_verify(&raw[1..]), "--zip-root")
                .unwrap_or_else(|| "harness-ultragoal-plugin-proposal".to_string()),
            archive_purpose: opt_string(strip_build_or_verify(&raw[1..]), "--archive-purpose")
                .or_else(|| opt_string(strip_build_or_verify(&raw[1..]), "--purpose"))
                .unwrap_or_else(|| "candidate_review_anchor".to_string()),
        },
        "review-round" => Command::ReviewRound {
            receipt: opt_path(strip_build_or_verify(&raw[1..]), "--receipt")?,
            validator_receipt: opt_path(strip_build_or_verify(&raw[1..]), "--validator-receipt")?,
            review_target_receipt: opt_path(
                strip_build_or_verify(&raw[1..]),
                "--review-target-receipt",
            )?,
            archive_receipt: opt_path(strip_build_or_verify(&raw[1..]), "--archive-receipt")?,
        },
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
        "product" => {
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
        _ => {
            if let Some(command) = cli::performance::parse(raw)? {
                Command::Performance(command)
            } else if let Some(command) = cli::rust::parse(raw)? {
                Command::Rust(command)
            } else if let Some(command) = cli::garbage::collection::parse(raw)? {
                Command::Garbage(command)
            } else if let Some(command) = cli::halo::parse(raw)? {
                Command::Halo(command)
            } else if let Some(command) = cli::improvement_loop::parse(raw)? {
                Command::ImprovementLoop(command)
            } else if let Some(command) = cli::observe::parse(raw)? {
                Command::Observe(command)
            } else if let Some(command) = cli::openai::parse(raw)? {
                Command::OpenAi(command)
            } else if let Some(command) = cli::promptfoo::parse(raw)? {
                Command::Promptfoo(command)
            } else if let Some(command) = cli::session::parse(raw)? {
                Command::Session(command)
            } else if let Some(command) = cli::control::plane::parse(raw) {
                Command::Control(command)
            } else {
                return Err(usage());
            }
        }
    })
}

fn strip_build_or_verify(args: &[String]) -> &[String] {
    match args.first().map(String::as_str) {
        Some("build" | "verify") => &args[1..],
        _ => args,
    }
}

fn parse_audit(args: &[String]) -> Result<Command, String> {
    Ok(Command::Audit {
        receipt: opt_path(args, "--receipt")?,
        red_report: opt_string(args, "--red-report").map(PathBuf::from),
        target_repo: opt_string(args, "--target-repo").map(PathBuf::from),
        mode: opt_string(args, "--mode").unwrap_or_else(|| "init".to_string()),
        require_observability: args.iter().any(|a| a == "--require-observability"),
        require_product_cohesion: args.iter().any(|a| a == "--require-product-cohesion"),
    })
}

fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    opt_string(args, key)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn usage() -> String {
    cli::usage::text().to_string()
}
