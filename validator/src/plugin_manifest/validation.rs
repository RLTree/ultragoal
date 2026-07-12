use super::ITEM_LIMIT;
use std::collections::{BTreeMap, BTreeSet};

const TEXT_LIMIT: usize = 16 * 1024;

pub(crate) fn bounded_text(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    !trimmed.is_empty()
        && value.len() <= TEXT_LIMIT
        && !matches!(
            lower.as_str(),
            "todo" | "tbd" | "placeholder" | "changeme" | "replace-me"
        )
        && !lower.contains("[todo:")
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

pub(crate) fn unique_list(values: &[String], maximum: usize) -> bool {
    values.len() <= maximum
        && values.iter().all(|value| bounded_text(value))
        && values
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            == values.len()
}

fn named_map(values: &BTreeMap<String, String>, valid_name: impl Fn(&str) -> bool) -> bool {
    values.len() <= ITEM_LIMIT
        && values
            .iter()
            .all(|(key, value)| valid_name(key) && bounded_text(value))
        && values
            .keys()
            .map(|key| key.to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            == values.len()
}

pub(crate) fn environment_map(values: &BTreeMap<String, String>) -> bool {
    named_map(values, |name| {
        name.len() <= 256
            && name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    })
}

pub(crate) fn header_map(values: &BTreeMap<String, String>) -> bool {
    named_map(values, |name| {
        name.len() <= 256
            && !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
    })
}

pub(crate) fn kebab(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

pub(crate) fn semver(value: &str) -> bool {
    super::Version::parse(value).is_some()
}

#[cfg(test)]
mod tests {
    use super::{environment_map, header_map};
    use std::collections::BTreeMap;

    #[test]
    fn mcp_environment_and_header_names_use_distinct_grammars() {
        for valid in ["MODE", "_MODE_2", "lower_case"] {
            assert!(environment_map(&BTreeMap::from([(
                valid.to_owned(),
                "value".to_owned()
            )])));
        }
        for invalid in ["", "1MODE", "BAD KEY", "A=B", "A:B", "BAD\r\nKEY"] {
            assert!(!environment_map(&BTreeMap::from([(
                invalid.to_owned(),
                "value".to_owned()
            )])));
        }
        for valid in ["Authorization", "X-Mode", "X_Proof", "X.Proof"] {
            assert!(header_map(&BTreeMap::from([(
                valid.to_owned(),
                "value".to_owned()
            )])));
        }
        for invalid in ["", "X Header", "X=Header", "X:Header", "X\r\nHeader"] {
            assert!(!header_map(&BTreeMap::from([(
                invalid.to_owned(),
                "value".to_owned()
            )])));
        }
        assert!(!environment_map(&BTreeMap::from([
            ("MODE".to_owned(), "one".to_owned()),
            ("mode".to_owned(), "two".to_owned())
        ])));
        assert!(!header_map(&BTreeMap::from([
            ("X-Mode".to_owned(), "one".to_owned()),
            ("x-mode".to_owned(), "two".to_owned())
        ])));
    }
}
