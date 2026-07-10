use super::fs::read_bounded;
use crate::context::ReadSession;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};

const MAX_SCHEMA_BYTES: u64 = 8 * 1024 * 1024;

enum NormalizedReference<'a> {
    Local(Option<&'a str>),
    Relative(&'a str, Option<&'a str>),
    Official(&'a str),
}

fn safe_fragment(fragment: Option<&str>) -> bool {
    fragment.is_none_or(|fragment| {
        (fragment.is_empty() || fragment.starts_with('/'))
            && fragment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"/_~.$-".contains(&byte))
    })
}

fn normalize(reference: &str) -> Result<NormalizedReference<'_>, ()> {
    if reference.is_empty()
        || reference.len() > 512
        || !reference.is_ascii()
        || reference.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(());
    }
    if let Some(fragment) = reference.strip_prefix('#') {
        return safe_fragment(Some(fragment))
            .then_some(NormalizedReference::Local(Some(fragment)))
            .ok_or(());
    }
    let (target, fragment) = reference
        .split_once('#')
        .map_or((reference, None), |(target, fragment)| {
            (target, Some(fragment))
        });
    if target.starts_with("https://json-schema.org/") {
        return (target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":/?._~-".contains(&byte))
            && safe_fragment(fragment))
        .then_some(NormalizedReference::Official(target))
        .ok_or(());
    }
    (target.ends_with(".json")
        && target.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"/._-".contains(&byte)
        })
        && Path::new(target)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && safe_fragment(fragment))
    .then_some(NormalizedReference::Relative(target, fragment))
    .ok_or(())
}

fn collect_raw(value: &Value, references: &mut BTreeSet<String>) {
    match value {
        Value::Object(values) => {
            if let Some(reference) = values.get("$ref").and_then(Value::as_str) {
                references.insert(reference.to_owned());
            }
            for value in values.values() {
                collect_raw(value, references);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_raw(value, references);
            }
        }
        _ => {}
    }
}

fn pointer_exists(value: &Value, fragment: Option<&str>) -> bool {
    fragment.is_none_or(|fragment| fragment.is_empty() || value.pointer(fragment).is_some())
}

pub(crate) struct JsonReferenceScan {
    pub references: Vec<String>,
    pub problem: Option<&'static str>,
}

pub(crate) fn json_references(reads: &ReadSession, path: &Path) -> JsonReferenceScan {
    let Ok(bytes) = read_bounded(reads, path, MAX_SCHEMA_BYTES) else {
        return JsonReferenceScan {
            references: Vec::new(),
            problem: Some("JSON reference source exceeds its read boundary"),
        };
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return JsonReferenceScan {
            references: Vec::new(),
            problem: Some("JSON reference source is invalid JSON"),
        };
    };
    let mut raw = BTreeSet::new();
    collect_raw(&value, &mut raw);
    let mut references = BTreeSet::new();
    let mut invalid = false;
    for reference in raw {
        match normalize(&reference) {
            Ok(NormalizedReference::Local(fragment)) => {
                invalid |= !pointer_exists(&value, fragment);
            }
            Ok(NormalizedReference::Official(target)) => {
                references.insert(target.to_owned());
            }
            Ok(NormalizedReference::Relative(target, fragment)) => {
                let Some(parent) = path.parent() else {
                    invalid = true;
                    continue;
                };
                let target_path = parent.join(target);
                let target_value = read_bounded(reads, &target_path, MAX_SCHEMA_BYTES)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
                if target_value
                    .as_ref()
                    .is_some_and(|value| pointer_exists(value, fragment))
                {
                    references.insert(target.to_owned());
                } else {
                    invalid = true;
                }
            }
            Err(()) => invalid = true,
        }
    }
    JsonReferenceScan {
        references: references.into_iter().collect(),
        problem: invalid.then_some("JSON reference is invalid, unsafe, or unresolved"),
    }
}
