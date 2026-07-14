use sha2::{Digest, Sha256};

use super::rust_source_syntax::{RustSourceSyntaxError, RustSourceSyntaxErrorKind, error};

pub(super) const FRAME_PREFIX: &[u8] = b"HUL-RSS-FRAME\0";
pub(super) const SCHEMA: &[u8] = b"rust-source-syntax-v1";
pub(super) const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_SOURCE_COUNT: usize = 512;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Clone, Copy)]
pub struct RustSourceFrameInput<'a> {
    path: &'a str,
    sha256: &'a str,
    byte_length: u64,
    bytes: &'a [u8],
}

impl<'a> RustSourceFrameInput<'a> {
    pub fn new(path: &'a str, sha256: &'a str, byte_length: u64, bytes: &'a [u8]) -> Self {
        Self {
            path,
            sha256,
            byte_length,
            bytes,
        }
    }
}

pub(super) struct ParsedSource<'a> {
    pub path: &'a str,
    pub digest: [u8; 32],
    pub bytes: &'a [u8],
}

pub fn encode_rust_source_syntax_frame(
    sources: &[RustSourceFrameInput<'_>],
) -> Result<Vec<u8>, RustSourceSyntaxError> {
    validate_count(sources.len())?;
    let mut frame = Vec::new();
    frame.extend_from_slice(FRAME_PREFIX);
    push_u16(&mut frame, SCHEMA.len())?;
    frame.extend_from_slice(SCHEMA);
    push_u32(&mut frame, sources.len())?;
    let mut prior_path = None;
    for source in sources {
        validate_ordered_path(source.path, prior_path)?;
        validate_source_length(source.bytes.len(), source.byte_length)?;
        let digest = parse_digest(source.sha256)?;
        if digest != Sha256::digest(source.bytes).as_slice() {
            return Err(error(RustSourceSyntaxErrorKind::DigestMismatch));
        }
        push_u16(&mut frame, source.path.len())?;
        frame.extend_from_slice(source.path.as_bytes());
        frame.extend_from_slice(&digest);
        frame.extend_from_slice(&source.byte_length.to_be_bytes());
        frame.extend_from_slice(source.bytes);
        if frame.len() > MAX_FRAME_BYTES {
            return Err(error(RustSourceSyntaxErrorKind::FrameTooLarge));
        }
        prior_path = Some(source.path);
    }
    Ok(frame)
}

pub(super) fn parse_frame(bytes: &[u8]) -> Result<Vec<ParsedSource<'_>>, RustSourceSyntaxError> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(error(RustSourceSyntaxErrorKind::FrameTooLarge));
    }
    let mut cursor = Cursor::new(bytes);
    if cursor.take(FRAME_PREFIX.len())? != FRAME_PREFIX {
        return Err(error(RustSourceSyntaxErrorKind::UnsupportedProtocol));
    }
    let schema_len = cursor.u16()? as usize;
    if cursor.take(schema_len)? != SCHEMA {
        return Err(error(RustSourceSyntaxErrorKind::UnsupportedProtocol));
    }
    let count = cursor.u32()? as usize;
    validate_count(count)?;
    let mut sources = Vec::with_capacity(count);
    let mut prior_path = None;
    for _ in 0..count {
        let path_len = cursor.u16()? as usize;
        let path = std::str::from_utf8(cursor.take(path_len)?)
            .map_err(|_| error(RustSourceSyntaxErrorKind::InvalidPathUtf8))?;
        validate_ordered_path(path, prior_path)?;
        let digest: [u8; 32] = cursor
            .take(32)?
            .try_into()
            .map_err(|_| error(RustSourceSyntaxErrorKind::TruncatedFrame))?;
        let declared_length = cursor.u64()?;
        let length = usize::try_from(declared_length)
            .map_err(|_| error(RustSourceSyntaxErrorKind::SourceTooLarge))?;
        validate_source_length(length, declared_length)?;
        let source_bytes = cursor.take(length)?;
        if digest != Sha256::digest(source_bytes).as_slice() {
            return Err(error(RustSourceSyntaxErrorKind::DigestMismatch));
        }
        sources.push(ParsedSource {
            path,
            digest,
            bytes: source_bytes,
        });
        prior_path = Some(path);
    }
    if cursor.remaining() != 0 {
        return Err(error(RustSourceSyntaxErrorKind::TrailingBytes));
    }
    Ok(sources)
}

fn validate_count(count: usize) -> Result<(), RustSourceSyntaxError> {
    if count == 0 || count > MAX_SOURCE_COUNT {
        Err(error(RustSourceSyntaxErrorKind::SourceCountOutOfBounds))
    } else {
        Ok(())
    }
}

fn validate_ordered_path(path: &str, prior: Option<&str>) -> Result<(), RustSourceSyntaxError> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || !path.ends_with(".rs")
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
    {
        return Err(error(RustSourceSyntaxErrorKind::InvalidPath));
    }
    if let Some(prior) = prior {
        if path == prior {
            return Err(error(RustSourceSyntaxErrorKind::DuplicatePath));
        }
        if path < prior {
            return Err(error(RustSourceSyntaxErrorKind::NonCanonicalOrder));
        }
    }
    Ok(())
}

fn validate_source_length(actual: usize, declared: u64) -> Result<(), RustSourceSyntaxError> {
    if actual > MAX_SOURCE_BYTES {
        Err(error(RustSourceSyntaxErrorKind::SourceTooLarge))
    } else if u64::try_from(actual).ok() != Some(declared) {
        Err(error(RustSourceSyntaxErrorKind::LengthMismatch))
    } else {
        Ok(())
    }
}

fn parse_digest(value: &str) -> Result<[u8; 32], RustSourceSyntaxError> {
    let hex = value
        .strip_prefix("sha256:")
        .filter(|hex| hex.len() == 64)
        .ok_or_else(|| error(RustSourceSyntaxErrorKind::InvalidDigest))?;
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| error(RustSourceSyntaxErrorKind::InvalidDigest))?;
        digest[index] = u8::from_str_radix(text, 16)
            .map_err(|_| error(RustSourceSyntaxErrorKind::InvalidDigest))?;
    }
    Ok(digest)
}

fn push_u16(frame: &mut Vec<u8>, value: usize) -> Result<(), RustSourceSyntaxError> {
    let value =
        u16::try_from(value).map_err(|_| error(RustSourceSyntaxErrorKind::IntegerOverflow))?;
    frame.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn push_u32(frame: &mut Vec<u8>, value: usize) -> Result<(), RustSourceSyntaxError> {
    let value =
        u32::try_from(value).map_err(|_| error(RustSourceSyntaxErrorKind::IntegerOverflow))?;
    frame.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], RustSourceSyntaxError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| error(RustSourceSyntaxErrorKind::TruncatedFrame))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, RustSourceSyntaxError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| error(RustSourceSyntaxErrorKind::TruncatedFrame))?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, RustSourceSyntaxError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| error(RustSourceSyntaxErrorKind::TruncatedFrame))?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, RustSourceSyntaxError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| error(RustSourceSyntaxErrorKind::TruncatedFrame))?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}
