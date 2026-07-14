use super::*;

pub(crate) fn write_staged(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "staged parent creation failed".to_owned())?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let mut file = options
        .open(path)
        .map_err(|_| "staged source creation failed".to_owned())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "staged source write failed".to_owned())?;
    let staged = fs::read(path).map_err(|_| "staged source verification failed".to_owned())?;
    if staged != bytes {
        return Err("staged source bytes differ".to_owned());
    }
    Ok(())
}

pub(crate) fn parse_authorable_templates(bytes: &[u8]) -> Result<Vec<String>, String> {
    let key = b"\"authorable_templates\"";
    let matches = bytes
        .windows(key.len())
        .filter(|window| *window == key)
        .count();
    if matches != 1 {
        return Err("manifest must contain one authorable_templates key".to_owned());
    }
    let key_start = bytes
        .windows(key.len())
        .position(|window| window == key)
        .ok_or_else(|| "manifest source list is absent".to_owned())?;
    let mut index = key_start + key.len();
    skip_whitespace(bytes, &mut index);
    expect(bytes, &mut index, b':')?;
    skip_whitespace(bytes, &mut index);
    expect(bytes, &mut index, b'[')?;
    let mut values = Vec::new();
    loop {
        skip_whitespace(bytes, &mut index);
        if bytes.get(index) == Some(&b']') {
            index += 1;
            break;
        }
        values.push(parse_string(bytes, &mut index)?);
        if values.len() > MAX_ROWS {
            return Err("manifest source count exceeded its bound".to_owned());
        }
        skip_whitespace(bytes, &mut index);
        match bytes.get(index) {
            Some(b',') => index += 1,
            Some(b']') => {
                index += 1;
                break;
            }
            _ => return Err("manifest source list is malformed".to_owned()),
        }
    }
    let _ = index;
    Ok(values)
}

pub(crate) fn parse_string(bytes: &[u8], index: &mut usize) -> Result<String, String> {
    expect(bytes, index, b'"')?;
    let mut output = Vec::new();
    while let Some(byte) = bytes.get(*index).copied() {
        *index += 1;
        match byte {
            b'"' => {
                return String::from_utf8(output)
                    .map_err(|_| "manifest source path is not UTF-8".to_owned());
            }
            b'\\' => {
                let escaped = bytes
                    .get(*index)
                    .copied()
                    .ok_or_else(|| "manifest string escape is truncated".to_owned())?;
                *index += 1;
                output.push(match escaped {
                    b'"' | b'\\' | b'/' => escaped,
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    _ => return Err("manifest source path uses an unsupported escape".to_owned()),
                });
            }
            0..=31 => return Err("manifest source path contains a control byte".to_owned()),
            _ => output.push(byte),
        }
    }
    Err("manifest source string is unterminated".to_owned())
}

pub(crate) fn validate_manifest_paths(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() || paths.len() > MAX_ROWS {
        return Err("manifest source count is outside the bounded contract".to_owned());
    }
    let mut exact = BTreeSet::new();
    let mut folded = BTreeSet::new();
    let mut previous = None;
    for path in paths {
        let target = path
            .strip_prefix("templates/")
            .ok_or_else(|| "manifest source is outside templates".to_owned())?;
        validate_relative_path(target)?;
        if previous.is_some_and(|value: &String| value >= path)
            || !exact.insert(path.clone())
            || !folded.insert(path.to_ascii_lowercase())
        {
            return Err("manifest source list is duplicate, aliased, or unordered".to_owned());
        }
        previous = Some(path);
    }
    Ok(())
}

pub(crate) fn validate_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 512
        || !value.is_ascii()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".." | "~"))
    {
        return Err("template source path is not canonical".to_owned());
    }
    Ok(())
}

pub(crate) fn skip_whitespace(bytes: &[u8], index: &mut usize) {
    while bytes
        .get(*index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
    {
        *index += 1;
    }
}

pub(crate) fn expect(bytes: &[u8], index: &mut usize, expected: u8) -> Result<(), String> {
    if bytes.get(*index) != Some(&expected) {
        return Err("manifest structure is malformed".to_owned());
    }
    *index += 1;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn identity(metadata: &fs::Metadata) -> SourceIdentity {
    SourceIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        length: metadata.len(),
        mode: metadata.permissions().mode() & 0o7777,
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
pub(crate) fn identity(_metadata: &fs::Metadata) -> SourceIdentity {
    SourceIdentity {
        device: 0,
        inode: 0,
        links: 0,
        length: 0,
        mode: 0,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
    }
}

#[cfg(target_os = "macos")]
pub(crate) const fn no_follow_nonblock_flags() -> i32 {
    0x0100 | 0x0004
}
