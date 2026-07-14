use super::plugin_manifest::{MAX_LIST_ITEMS, bounded_text};
use crate::context::ReadSession;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

include!("max_timeout_seconds.rs");

include!("inspect.rs");

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
