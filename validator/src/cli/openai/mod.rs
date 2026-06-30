use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) mod budget;
mod call;
mod config;
mod live;
pub(crate) mod output;
pub(crate) mod policy;

pub(crate) const LAW_ID: &str = "openai-api-key-model-cost-external-ai-boundary";
pub(crate) const RECEIPT_SCHEMA: &str = "harness-ultragoal.openai-config-receipt.v1";

#[derive(Debug)]
pub(crate) enum OpenAiCommand {
    Config(config::ConfigCommand),
    Call(call::CallCommand),
    Output(output::OutputCommand),
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<OpenAiCommand>, String> {
    if raw.first().map(String::as_str) != Some("openai") {
        return Ok(None);
    }
    match raw {
        [_, action, subject, ..] if action == "config" && subject == "prove" => {
            Ok(Some(OpenAiCommand::Config(config::ConfigCommand {
                receipt: opt_path(raw, "--receipt")
                    .unwrap_or_else(|| PathBuf::from(config::DEFAULT_RECEIPT)),
                policy: opt_path(raw, "--policy")
                    .unwrap_or_else(|| PathBuf::from(config::DEFAULT_POLICY)),
            })))
        }
        [_, action, subject, ..] if action == "call" && subject == "prove" => {
            Ok(Some(OpenAiCommand::Call(call::parse(raw)?)))
        }
        [_, action, subject, ..] if action == "output" && subject == "prove" => {
            Ok(Some(OpenAiCommand::Output(output::parse(raw))))
        }
        _ => Err("unknown ultragoal openai command".to_string()),
    }
}

pub(crate) fn run(root: &Path, command: &OpenAiCommand) -> Result<i32, String> {
    match command {
        OpenAiCommand::Config(config) => config::run(root, config),
        OpenAiCommand::Call(call) => call::run(root, call),
        OpenAiCommand::Output(output) => output::run(root, output),
    }
}

#[cfg(test)]
pub(crate) fn build_receipt(root: &Path, command: &OpenAiCommand) -> Result<Value, String> {
    match command {
        OpenAiCommand::Config(config) => config::build_config_receipt(root, config),
        OpenAiCommand::Call(call) => call::build_call_receipt(root, call),
        OpenAiCommand::Output(output) => output::build_output_receipt(root, output),
    }
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn csv(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| "none".to_string())
}
