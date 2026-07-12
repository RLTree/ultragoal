fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn fixed_hex(bytes: &[u8], cursor: &mut usize, digits: usize) -> Result<u32, ()> {
    let end = cursor
        .checked_add(digits)
        .filter(|end| *end <= bytes.len())
        .ok_or(())?;
    let mut value = 0_u32;
    for byte in &bytes[*cursor..end] {
        value = value.checked_mul(16).ok_or(())? + hex_value(*byte).ok_or(())?;
    }
    *cursor = end;
    Ok(value)
}

fn push_scalar(decoded: &mut Vec<u8>, value: u32) -> Result<(), ()> {
    let value = char::from_u32(value).ok_or(())?;
    let mut encoded = [0_u8; 4];
    decoded.extend_from_slice(value.encode_utf8(&mut encoded).as_bytes());
    Ok(())
}

pub(super) fn decode_escape(
    bytes: &[u8],
    cursor: &mut usize,
    decoded: &mut Vec<u8>,
) -> Result<(), ()> {
    let escaped = *bytes.get(*cursor).ok_or(())?;
    *cursor += 1;
    match escaped {
        b'\n' => {}
        b'\r' => {
            if bytes.get(*cursor) == Some(&b'\n') {
                *cursor += 1;
            }
        }
        b'x' => push_scalar(decoded, fixed_hex(bytes, cursor, 2)?)?,
        b'u' => {
            if bytes.get(*cursor) == Some(&b'{') {
                *cursor += 1;
                let start = *cursor;
                let mut value = 0_u32;
                while let Some(byte) = bytes.get(*cursor).copied() {
                    if byte == b'}' {
                        break;
                    }
                    if *cursor - start == 6 {
                        return Err(());
                    }
                    value = value.checked_mul(16).ok_or(())? + hex_value(byte).ok_or(())?;
                    *cursor += 1;
                }
                if *cursor == start || bytes.get(*cursor) != Some(&b'}') {
                    return Err(());
                }
                *cursor += 1;
                push_scalar(decoded, value)?;
            } else {
                while bytes.get(*cursor) == Some(&b'u') {
                    *cursor += 1;
                }
                push_scalar(decoded, fixed_hex(bytes, cursor, 4)?)?;
            }
        }
        b'U' => push_scalar(decoded, fixed_hex(bytes, cursor, 8)?)?,
        b'0'..=b'7' => {
            let mut value = u32::from(escaped - b'0');
            for _ in 0..2 {
                let Some(next @ b'0'..=b'7') = bytes.get(*cursor).copied() else {
                    break;
                };
                value = value * 8 + u32::from(next - b'0');
                *cursor += 1;
            }
            push_scalar(decoded, value)?;
        }
        b'a' => decoded.push(0x07),
        b'b' => decoded.push(0x08),
        b'f' => decoded.push(0x0c),
        b'n' => decoded.push(b'\n'),
        b'r' => decoded.push(b'\r'),
        b't' => decoded.push(b'\t'),
        b'v' => decoded.push(0x0b),
        other => decoded.push(other),
    }
    Ok(())
}

pub(super) fn java_unicode_source(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    let mut changed = false;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1) == Some(&b'u') {
            let mut digits = cursor + 2;
            while bytes.get(digits) == Some(&b'u') {
                digits += 1;
            }
            let mut after = digits;
            if let Ok(value) = fixed_hex(bytes, &mut after, 4)
                && let Some(value) = char::from_u32(value)
            {
                let mut encoded = [0_u8; 4];
                decoded.extend_from_slice(value.encode_utf8(&mut encoded).as_bytes());
                cursor = after;
                changed = true;
                continue;
            }
        }
        decoded.push(bytes[cursor]);
        cursor += 1;
    }
    changed.then_some(decoded)
}
