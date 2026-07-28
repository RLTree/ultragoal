use super::cfg;
mod lex;

use lex::{
    identifier_end, identifier_start, mask, matching, skip_ascii_space, skip_trivia, skipped_token,
};
use syn::parse::Parser;

pub(super) fn production_text(relative: &str, text: &str) -> Result<String, String> {
    let mut bytes = text.as_bytes().to_vec();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if let Some(end) = skipped_token(&bytes, cursor) {
            cursor = end;
            continue;
        }
        if bytes[cursor] != b'#' || bytes.get(cursor + 1) == Some(&b'!') {
            cursor += 1;
            continue;
        }
        let Some((attributes, group_end)) = attribute_group(text, cursor) else {
            cursor += 1;
            continue;
        };
        let possibility = cfg::possibility(&attributes)
            .map_err(|detail| format!("production_source_cfg_invalid:{relative}:{detail}"))?;
        if possibility.production {
            cursor = group_end;
            continue;
        }
        let construct_end = attached_construct_end(&bytes, group_end)
            .ok_or_else(|| format!("production_source_filter_boundary_invalid:{relative}"))?;
        mask(&mut bytes[cursor..construct_end]);
        cursor = construct_end;
    }
    let filtered = String::from_utf8(bytes)
        .map_err(|_| format!("production_source_filter_utf8_invalid:{relative}"))?;
    syn::parse_file(&filtered)
        .map_err(|_| format!("production_source_filter_syntax_invalid:{relative}"))?;
    Ok(filtered)
}

fn attribute_group(text: &str, start: usize) -> Option<(Vec<syn::Attribute>, usize)> {
    let bytes = text.as_bytes();
    let mut attributes = Vec::new();
    let mut cursor = start;
    loop {
        let end = attribute_end(bytes, cursor)?;
        let parser = syn::Attribute::parse_outer;
        let mut parsed = parser.parse_str(&text[cursor..end]).ok()?;
        if parsed.len() != 1 {
            return None;
        }
        attributes.push(parsed.remove(0));
        cursor = skip_trivia(bytes, end);
        if bytes.get(cursor) != Some(&b'#') || bytes.get(cursor + 1) == Some(&b'!') {
            return Some((attributes, cursor));
        }
    }
}

fn attribute_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'#') {
        return None;
    }
    let open = skip_ascii_space(bytes, start + 1);
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let mut depth = 1usize;
    let mut cursor = open + 1;
    while cursor < bytes.len() {
        if let Some(end) = skipped_token(bytes, cursor) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor + 1);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn attached_construct_end(bytes: &[u8], start: usize) -> Option<usize> {
    let start = skip_trivia(bytes, start);
    let braced_item = braced_declaration(bytes, start);
    let mut stack = Vec::new();
    let mut cursor = start;
    while cursor < bytes.len() {
        if let Some(end) = skipped_token(bytes, cursor) {
            cursor = end;
            continue;
        }
        let byte = bytes[cursor];
        match byte {
            b'(' | b'[' | b'{' => stack.push(byte),
            b')' | b']' | b'}' => {
                if stack.is_empty() {
                    return Some(cursor);
                }
                let open = stack.pop()?;
                if !matching(open, byte) {
                    return None;
                }
                if stack.is_empty() && open == b'{' && braced_item {
                    let after = skip_trivia(bytes, cursor + 1);
                    return Some(if bytes.get(after) == Some(&b';') {
                        after + 1
                    } else {
                        cursor + 1
                    });
                }
            }
            b';' | b',' if stack.is_empty() => return Some(cursor + 1),
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn braced_declaration(bytes: &[u8], start: usize) -> bool {
    let mut cursor = start;
    while cursor < bytes.len() {
        if let Some(end) = skipped_token(bytes, cursor) {
            cursor = end;
            continue;
        }
        if matches!(bytes[cursor], b'{' | b';' | b',') {
            return false;
        }
        if identifier_start(bytes[cursor]) {
            let end = identifier_end(bytes, cursor);
            let word = std::str::from_utf8(&bytes[cursor..end]).unwrap_or("");
            if matches!(
                word,
                "fn" | "mod" | "impl" | "trait" | "enum" | "struct" | "union" | "extern"
            ) {
                return true;
            }
            if !matches!(
                word,
                "pub" | "crate" | "super" | "self" | "in" | "unsafe" | "async" | "const"
            ) {
                return false;
            }
            cursor = end;
        } else {
            cursor += 1;
        }
    }
    false
}
