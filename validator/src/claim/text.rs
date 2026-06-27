use std::collections::BTreeSet;

pub fn normalized_text(parts: &[&str]) -> String {
    let raw = parts.join(" ");
    let mut expanded = String::new();
    let mut prev_lower_or_digit = false;
    for ch in raw.chars() {
        if ch.is_ascii_uppercase() && prev_lower_or_digit {
            expanded.push(' ');
        }
        prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        expanded.push(ch.to_ascii_lowercase());
    }
    let mut out = String::new();
    let mut last_space = true;
    for ch in expanded.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    format!(" {} ", out.trim())
}

pub fn tokens(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for token in text.split_whitespace() {
        out.insert(token.to_string());
        if token.len() > 3 && token.ends_with('s') {
            out.insert(token.trim_end_matches('s').to_string());
        }
    }
    out
}

pub fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.contains(&format!(" {} ", normalized_text(&[phrase]).trim()))
}

pub fn contains_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| contains_phrase(text, phrase))
}
