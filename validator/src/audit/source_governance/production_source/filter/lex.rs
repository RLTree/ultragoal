pub(super) fn skipped_token(bytes: &[u8], start: usize) -> Option<usize> {
    match (bytes.get(start), bytes.get(start + 1)) {
        (Some(b'/'), Some(b'/')) => Some(line_comment_end(bytes, start + 2)),
        (Some(b'/'), Some(b'*')) => block_comment_end(bytes, start),
        (Some(b'"'), _) => quoted_end(bytes, start, b'"'),
        (Some(b'\''), _) => quoted_end(bytes, start, b'\''),
        (Some(b'r'), _) => raw_string_end(bytes, start),
        (Some(b'b'), Some(b'"')) => quoted_end(bytes, start + 1, b'"'),
        (Some(b'b'), Some(b'\'')) => quoted_end(bytes, start + 1, b'\''),
        (Some(b'b'), Some(b'r')) => raw_string_end(bytes, start + 1),
        _ => None,
    }
}

pub(super) fn skip_trivia(bytes: &[u8], mut cursor: usize) -> usize {
    loop {
        cursor = skip_ascii_space(bytes, cursor);
        match skipped_token(bytes, cursor) {
            Some(end) if bytes.get(cursor) == Some(&b'/') => cursor = end,
            _ => return cursor,
        }
    }
}

pub(super) fn skip_ascii_space(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    cursor
}

pub(super) fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

pub(super) fn identifier_end(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        cursor += 1;
    }
    cursor
}

pub(super) fn matching(open: u8, close: u8) -> bool {
    matches!((open, close), (b'(', b')') | (b'[', b']') | (b'{', b'}'))
}

pub(super) fn mask(bytes: &mut [u8]) {
    for byte in bytes {
        if *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
}

fn line_comment_end(bytes: &[u8], mut cursor: usize) -> usize {
    while cursor < bytes.len() && bytes[cursor] != b'\n' {
        cursor += 1;
    }
    cursor
}

fn block_comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = start + 2;
    while cursor + 1 < bytes.len() {
        match (&bytes[cursor..cursor + 2], depth) {
            (b"/*", _) => {
                depth += 1;
                cursor += 2;
            }
            (b"*/", 1) => return Some(cursor + 2),
            (b"*/", _) => {
                depth -= 1;
                cursor += 2;
            }
            _ => cursor += 1,
        }
    }
    None
}

fn quoted_end(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor += 2;
        } else if bytes[cursor] == quote {
            return Some(cursor + 1);
        } else {
            cursor += 1;
        }
    }
    None
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut cursor = start + 1;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    let hashes = cursor - start - 1;
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes)
                == Some(&bytes[start + 1..start + 1 + hashes])
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    None
}
