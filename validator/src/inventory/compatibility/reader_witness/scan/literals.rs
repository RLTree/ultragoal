use super::unicode::decode_escape;

struct LiteralFragment {
    compact: Vec<u8>,
    dot_before: Vec<usize>,
    path_boundary_before: Vec<usize>,
}

fn fragment(decoded: &[u8]) -> LiteralFragment {
    let mut compact = Vec::new();
    let mut dot_before = Vec::new();
    let mut path_boundary_before = Vec::new();
    let mut pending_dot = false;
    let mut pending_path_boundary = false;
    for byte in decoded {
        let lower = byte.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            if pending_dot {
                dot_before.push(compact.len());
            }
            if pending_path_boundary {
                path_boundary_before.push(compact.len());
            }
            compact.push(lower);
            pending_dot = false;
            pending_path_boundary = false;
        } else {
            pending_dot |= *byte == b'.';
            pending_path_boundary |= matches!(*byte, b'/' | b'\\' | b'.');
        }
    }
    LiteralFragment {
        compact,
        dot_before,
        path_boundary_before,
    }
}

fn quoted_literal(
    bytes: &[u8],
    mut cursor: usize,
    quote: u8,
    triple: bool,
) -> Result<(LiteralFragment, usize), ()> {
    let mut decoded = Vec::new();
    while cursor < bytes.len() {
        if triple {
            if bytes.get(cursor..cursor + 3) == Some([quote, quote, quote].as_slice()) {
                return Ok((fragment(&decoded), cursor + 3));
            }
        } else if bytes[cursor] == quote {
            return Ok((fragment(&decoded), cursor + 1));
        }
        if bytes[cursor] == b'\\' {
            cursor += 1;
            decode_escape(bytes, &mut cursor, &mut decoded)?;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    Err(())
}

fn raw_literal(bytes: &[u8], start: usize) -> Option<Result<(LiteralFragment, usize), ()>> {
    if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        return None;
    }
    let mut cursor = match bytes.get(start..) {
        Some([b'r', ..]) => start + 1,
        Some([b'b', b'r', ..]) => start + 2,
        _ => return None,
    };
    let hash_start = cursor;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    let hashes = cursor - hash_start;
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    cursor += 1;
    let content_start = cursor;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes)
                == Some(&bytes[hash_start..hash_start + hashes])
        {
            return Some(Ok((
                fragment(&bytes[content_start..cursor]),
                cursor + 1 + hashes,
            )));
        }
        cursor += 1;
    }
    Some(Err(()))
}

fn lifetime_end(bytes: &[u8], quote: usize) -> Option<usize> {
    let mut cursor = quote + 1;
    if !bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        return None;
    }
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'\'') {
        return None;
    }
    let prior = bytes[..quote]
        .iter()
        .rfind(|byte| !byte.is_ascii_whitespace())
        .copied();
    let next = bytes.get(cursor).copied();
    (matches!(prior, Some(b'&' | b'<' | b':' | b'+'))
        || matches!(next, Some(b'>' | b',' | b':' | b'+')))
    .then_some(cursor)
}

fn literal_fragments(bytes: &[u8]) -> Result<Vec<LiteralFragment>, ()> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if let Some(raw) = raw_literal(bytes, cursor) {
            let (fragment, next) = raw?;
            out.push(fragment);
            cursor = next;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            cursor += 2;
            while bytes.get(cursor).is_some_and(|byte| *byte != b'\n') {
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            cursor += 2;
            let mut depth = 1_u32;
            while cursor < bytes.len() && depth > 0 {
                if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            continue;
        }
        if bytes[cursor] == b'#'
            && cursor
                .checked_sub(1)
                .is_none_or(|prior| bytes[prior].is_ascii_whitespace())
        {
            while bytes.get(cursor).is_some_and(|byte| *byte != b'\n') {
                cursor += 1;
            }
            continue;
        }
        let quote = bytes[cursor];
        if matches!(quote, b'"' | b'\'' | b'`') {
            if quote == b'\''
                && let Some(next) = lifetime_end(bytes, cursor)
            {
                cursor = next;
                continue;
            }
            let triple = quote != b'`'
                && bytes.get(cursor..cursor + 3) == Some([quote, quote, quote].as_slice());
            let opening = if triple { 3 } else { 1 };
            let (fragment, next) = quoted_literal(bytes, cursor + opening, quote, triple)?;
            out.push(fragment);
            cursor = next;
        } else {
            cursor += 1;
        }
    }
    Ok(out)
}

fn literal_subsequence_starts_with(
    fragments: &[LiteralFragment],
    target: &[u8],
    canonical_start: Option<usize>,
) -> bool {
    let mut states = vec![(0_usize, false)];
    for fragment in fragments {
        let prior = states.clone();
        for (matched, has_dot) in prior {
            let remaining = &target[matched..];
            let candidate = &fragment.compact;
            let compared = remaining.len().min(candidate.len());
            if compared == 0 || candidate[..compared] != remaining[..compared] {
                continue;
            }
            if candidate.len() > remaining.len()
                && !fragment.path_boundary_before.contains(&compared)
            {
                continue;
            }
            let next = matched + compared;
            let next_has_dot = has_dot
                || canonical_start.is_some_and(|canonical_start| {
                    (matched..next).contains(&canonical_start)
                        && fragment.dot_before.contains(&(canonical_start - matched))
                });
            if next == target.len() && (canonical_start.is_none() || next_has_dot) {
                return true;
            }
            let state = (next, next_has_dot);
            if !states.contains(&state) {
                states.push(state);
            }
        }
    }
    false
}

pub(super) fn fragments_construct_agent_path(bytes: &[u8]) -> bool {
    let Ok(fragments) = literal_fragments(bytes) else {
        return true;
    };
    literal_subsequence_starts_with(&fragments, b"codexagents", Some(0))
        || literal_subsequence_starts_with(&fragments, b"agentscodex", Some(6))
        || literal_subsequence_starts_with(&fragments, b"customagents", None)
        || literal_subsequence_starts_with(&fragments, b"agentscustom", None)
}
