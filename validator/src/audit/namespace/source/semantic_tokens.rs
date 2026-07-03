pub(super) fn semantic_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;
    for ch in input.chars() {
        if !ch.is_ascii_alphanumeric() {
            push_token(&mut tokens, &mut current);
            previous = None;
            continue;
        }
        let starts_new = previous.is_some_and(|prev| {
            (ch.is_ascii_uppercase() && (prev.is_ascii_lowercase() || prev.is_ascii_digit()))
                || (ch.is_ascii_digit() && prev.is_ascii_alphabetic())
                || (ch.is_ascii_alphabetic() && prev.is_ascii_digit())
        });
        if starts_new {
            push_token(&mut tokens, &mut current);
        }
        current.push(ch.to_ascii_lowercase());
        previous = Some(ch);
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}
