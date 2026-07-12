use super::plugin_manifest::{MAX_LIST_ITEMS, bounded_text};
use crate::context::ReadSession;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_TIMEOUT_SECONDS: u64 = 86_400;

pub(super) type HookMap = BTreeMap<String, Vec<HookGroup>>;

pub(super) struct Inspection {
    pub(super) valid: bool,
    pub(super) inactive: bool,
    pub(super) trust_required: bool,
    pub(super) paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HookGroup {
    #[serde(default)]
    matcher: Option<String>,
    hooks: Vec<HookHandler>,
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum HookHandler {
    #[serde(rename = "command")]
    Command {
        command: String,
        #[serde(default, rename = "commandWindows")]
        command_windows: Option<String>,
        #[serde(default)]
        timeout: Option<u64>,
        #[serde(default, rename = "statusMessage")]
        status_message: Option<String>,
        #[serde(default, rename = "async")]
        asynchronous: Option<bool>,
    },
    #[serde(rename = "prompt")]
    Prompt { prompt: String },
    #[serde(rename = "agent")]
    Agent { agent: String },
}

fn supported_event(event: &str) -> bool {
    matches!(
        event,
        "SessionStart"
            | "SubagentStart"
            | "PreToolUse"
            | "PermissionRequest"
            | "PostToolUse"
            | "PreCompact"
            | "PostCompact"
            | "UserPromptSubmit"
            | "SubagentStop"
            | "Stop"
    )
}

fn handler_valid(handler: &HookHandler) -> bool {
    match handler {
        HookHandler::Command {
            command,
            command_windows,
            timeout,
            status_message,
            asynchronous,
        } => {
            bounded_text(command)
                && command_windows.as_deref().is_none_or(bounded_text)
                && timeout.is_none_or(|seconds| (1..=MAX_TIMEOUT_SECONDS).contains(&seconds))
                && status_message.as_deref().is_none_or(bounded_text)
                && asynchronous.is_none_or(|value| !value)
        }
        HookHandler::Prompt { prompt } => bounded_text(prompt),
        HookHandler::Agent { agent } => bounded_text(agent),
    }
}

fn map_valid(hooks: &HookMap) -> bool {
    !hooks.is_empty()
        && hooks.len() <= MAX_LIST_ITEMS
        && hooks.iter().all(|(event, groups)| {
            supported_event(event)
                && !groups.is_empty()
                && groups.len() <= MAX_LIST_ITEMS
                && groups.iter().all(|group| {
                    group
                        .matcher
                        .as_deref()
                        .is_none_or(super::plugin_manifest_hook_matcher::valid)
                        && !group.hooks.is_empty()
                        && group.hooks.len() <= MAX_LIST_ITEMS
                        && group.hooks.iter().all(handler_valid)
                })
        })
}

fn map_inactive(hooks: &HookMap) -> bool {
    hooks.iter().any(|(event, groups)| {
        groups.iter().any(|group| {
            (matches!(event.as_str(), "UserPromptSubmit" | "Stop") && group.matcher.is_some())
                || group.hooks.iter().any(|handler| {
                    matches!(
                        handler,
                        HookHandler::Prompt { .. } | HookHandler::Agent { .. }
                    )
                })
        })
    })
}

fn map_trust_required(hooks: &HookMap) -> bool {
    hooks.values().flatten().any(|group| {
        group
            .hooks
            .iter()
            .any(|handler| matches!(handler, HookHandler::Command { .. }))
    })
}

pub(super) fn inspect_map(hooks: &HookMap) -> Inspection {
    let valid = map_valid(hooks);
    Inspection {
        valid,
        inactive: valid && map_inactive(hooks),
        trust_required: valid && map_trust_required(hooks),
        paths: Vec::new(),
    }
}

fn inline_valid(value: &Value) -> bool {
    inline_inspect(value).valid
}

fn inline_inspect(value: &Value) -> Inspection {
    serde_json::from_value::<HookMap>(value.clone()).map_or(
        Inspection {
            valid: false,
            inactive: false,
            trust_required: false,
            paths: Vec::new(),
        },
        |hooks| inspect_map(&hooks),
    )
}

fn combine(inspections: impl Iterator<Item = Inspection>) -> Inspection {
    let mut combined = inspections.fold(
        Inspection {
            valid: true,
            inactive: false,
            trust_required: false,
            paths: Vec::new(),
        },
        |mut state, inspection| {
            state.valid &= inspection.valid;
            state.inactive |= inspection.inactive;
            state.trust_required |= inspection.trust_required;
            state.paths.extend(inspection.paths);
            state
        },
    );
    let path_count = combined.paths.len();
    combined.paths.sort();
    combined.paths.dedup();
    let valid = combined.valid && combined.paths.len() == path_count;
    Inspection {
        valid,
        inactive: valid && combined.inactive,
        trust_required: valid && combined.trust_required,
        paths: combined.paths,
    }
}

pub(super) fn inspect(reads: &ReadSession, root: &Path, value: &Value) -> Inspection {
    match value {
        Value::String(path) => super::plugin_manifest_hook_document::inspect(reads, root, path),
        Value::Object(_) => inline_inspect(value),
        Value::Array(items) if !items.is_empty() && items.len() <= MAX_LIST_ITEMS => {
            let paths = items.iter().all(Value::is_string);
            let inline = items.iter().all(Value::is_object);
            if paths {
                combine(items.iter().map(|item| {
                    super::plugin_manifest_hook_document::inspect(
                        reads,
                        root,
                        item.as_str().unwrap_or_default(),
                    )
                }))
            } else if inline {
                combine(items.iter().map(inline_inspect))
            } else {
                Inspection {
                    valid: false,
                    inactive: false,
                    trust_required: false,
                    paths: Vec::new(),
                }
            }
        }
        _ => Inspection {
            valid: false,
            inactive: false,
            trust_required: false,
            paths: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    fn command() -> Value {
        json!({"type":"command","command":"true","timeout":5,"statusMessage":"fixture"})
    }

    #[test]
    fn inline_hooks_require_event_group_and_handler_structure() {
        assert!(super::inline_valid(&json!({
            "SessionStart":[{"matcher":"startup","hooks":[command()]}]
        })));
        for invalid in [
            json!({}),
            json!({"UnknownEvent":[{"hooks":[command()]}]}),
            json!({"SessionStart":"not-an-array"}),
            json!({"SessionStart":[{"type":"command","command":"true"}]}),
            json!({"SessionStart":[{"hooks":[]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"command"}]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"unknown","command":"true"}]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"command","command":"true","extra":true}]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"command","command":"true","timeout":0}]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"command","command":"true","async":true}]}]}),
            json!({"SessionStart":[{"matcher":"[","hooks":[command()]}]}),
        ] {
            assert!(!super::inline_valid(&invalid), "{invalid}");
        }
    }

    #[test]
    fn parsed_but_skipped_hook_configuration_is_explicitly_inactive() {
        for value in [
            json!({"SessionStart":[{"hooks":[{"type":"prompt","prompt":"fixture"}]}]}),
            json!({"SessionStart":[{"hooks":[{"type":"agent","agent":"fixture"}]}]}),
            json!({"UserPromptSubmit":[{"matcher":"Bash","hooks":[command()]}]}),
            json!({"Stop":[{"matcher":"*","hooks":[command()]}]}),
        ] {
            let inspection = super::inline_inspect(&value);
            assert!(inspection.valid, "{value}");
            assert!(inspection.inactive, "{value}");
        }
    }
}
