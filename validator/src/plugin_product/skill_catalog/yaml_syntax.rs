pub(super) fn split_mapping(value: &str) -> Option<(&str, &str)> {
    let mut quote = None;
    for (index, byte) in value.bytes().enumerate() {
        match (quote, byte) {
            (None, b'\'' | b'"') => quote = Some(byte),
            (Some(active), byte) if byte == active => quote = None,
            (None, b':') => return Some((&value[..index], &value[index + 1..])),
            _ => {}
        }
    }
    None
}

pub(super) fn strip_comment(value: &str) -> &str {
    let mut quote = None;
    for (index, byte) in value.bytes().enumerate() {
        match (quote, byte) {
            (None, b'\'' | b'"') => quote = Some(byte),
            (Some(active), byte) if byte == active => quote = None,
            (None, b'#') if index == 0 || value.as_bytes()[index - 1].is_ascii_whitespace() => {
                return value[..index].trim_end();
            }
            _ => {}
        }
    }
    value
}

pub(super) fn scalar(value: &str) -> Result<String, ()> {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return serde_json::from_str(value).map_err(|_| ());
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return Ok(value[1..value.len() - 1].replace("''", "'"));
    }
    if value.starts_with(['[', '{', '&', '*', '!', '%', '@']) {
        return Err(());
    }
    Ok(value.to_owned())
}
