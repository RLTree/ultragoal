fn bounded(value: &str) -> bool {
    value.len() <= 16 * 1024
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn alternative_valid(value: &str) -> bool {
    let value = value.strip_prefix('^').unwrap_or(value);
    let value = value.strip_suffix('$').unwrap_or(value);
    if value.is_empty() {
        return false;
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'-') {
            index += 1;
        } else if bytes[index] == b'.' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
        } else {
            return false;
        }
    }
    true
}

// Codex consumes a regex here. Keep validation dependency-free and fail closed to
// the documented literal, anchored-literal, alternation, and `.*` matcher forms.
pub(super) fn valid(value: &str) -> bool {
    bounded(value) && (value.is_empty() || value == "*" || value.split('|').all(alternative_valid))
}

#[cfg(test)]
mod tests {
    #[test]
    fn validation_is_a_fail_closed_subset_of_documented_regex_forms() {
        for valid in [
            "",
            "*",
            "Bash",
            "^apply_patch$",
            "Edit|Write",
            "mcp__filesystem__.*",
            "startup|resume|clear|compact",
        ] {
            assert!(super::valid(valid), "{valid}");
        }
        for invalid in ["[", "(Bash)", "Bash?", "a||b", "^$", "\\w+"] {
            assert!(!super::valid(invalid), "{invalid}");
        }
    }
}
