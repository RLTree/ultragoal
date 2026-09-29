//! Descriptor-bound logical source capture in UTF-8 transport pages.
use super::*;
use crate::relative;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub enum StreamError {
    InvalidPath,
    Unsupported,
    Changed,
    Pressure { offset: u64 },
    Cancelled,
    Unavailable(i32),
    Consumer(String),
}

impl From<Problem> for StreamError {
    fn from(problem: Problem) -> Self {
        match problem {
            Problem::InvalidPath => Self::InvalidPath,
            Problem::Unstable => Self::Changed,
            Problem::Unsupported => Self::Unsupported,
            Problem::Unavailable(code) => Self::Unavailable(code),
            _ => Self::InvalidPath,
        }
    }
}

#[derive(Debug)]
pub struct StreamedSource {
    pub identity: Identity,
    pub digest: String,
    pub bytes: u64,
    pub pages: u64,
}

fn pressure(offset: u64, size: u64) -> Result<usize, StreamError> {
    let host = crate::resources::current().ok_or(StreamError::Pressure { offset })?;
    // Bend reassembles a logical file for grammar parsing. Its observed peak is
    // much larger than source bytes; admit only when current work headroom can
    // carry the whole logical file, and retry from a fresh source under recovery.
    if host.work_bytes() == 0 || size > host.work_bytes() / 32 {
        return Err(StreamError::Pressure { offset });
    }
    Ok(host.frame_bytes(crate::MAX) / 4)
}

/// Keep the descriptor and already acknowledged Bend pages while transient
/// host pressure clears. A persistent shortage returns the exact next offset.
fn await_pressure(offset: u64, size: u64, read: impl FnMut(u64, u64) -> Result<usize, StreamError>) -> Result<usize, StreamError> {
    await_pressure_for(offset, size, std::time::Duration::from_secs(10), read)
}
fn await_pressure_for(offset: u64, size: u64, budget: std::time::Duration, mut read: impl FnMut(u64, u64) -> Result<usize, StreamError>) -> Result<usize, StreamError> {
    let started = std::time::Instant::now();
    loop {
        if crate::is_interrupted() { return Err(StreamError::Cancelled); }
        match read(offset, size) {
            Err(StreamError::Pressure { .. }) if started.elapsed() < budget =>
                std::thread::sleep(std::time::Duration::from_millis(100)),
            result => return result,
        }
    }
}

fn delivered(
    pending: &mut Vec<u8>,
    offset: &mut u64,
    pages: &mut u64,
    each: &mut impl FnMut(u64, &str) -> Result<(), String>,
) -> Result<(), StreamError> {
    match std::str::from_utf8(pending) {
        Ok(text) => {
            if !text.is_empty() {
                each(*offset, text).map_err(StreamError::Consumer)?;
                *offset += pending.len() as u64;
                *pages += 1;
            }
            pending.clear();
        }
        Err(error) if error.error_len().is_none() => {
            let valid = error.valid_up_to();
            if valid > 0 {
                let text = std::str::from_utf8(&pending[..valid]).map_err(|_| StreamError::Unsupported)?;
                each(*offset, text).map_err(StreamError::Consumer)?;
                *offset += valid as u64;
                *pages += 1;
                pending.drain(..valid);
            }
            if pending.len() > 3 {
                return Err(StreamError::Unsupported);
            }
        }
        Err(_) => return Err(StreamError::Unsupported),
    }
    Ok(())
}

impl Root {
    /// Two passes over one held regular descriptor: first stream SHA-256, then
    /// deliver UTF-8 pages synchronously with backpressure. Final descriptor,
    /// directory lineage and bound path identity must all still match.
    pub fn stream_source(
        &self,
        path: &[u8],
        mut begin: impl FnMut(&StreamedSource) -> Result<(), String>,
        mut each: impl FnMut(u64, &str) -> Result<(), String>,
    ) -> Result<StreamedSource, StreamError> {
        if path.len() > HARD_MAX_PATH_BYTES {
            return Err(StreamError::InvalidPath);
        }
        let parts = relative::components(path).map_err(|_| StreamError::InvalidPath)?;
        let (leaf, parent) = parts.split_last().ok_or(StreamError::InvalidPath)?;
        let (directory, lineage) = self.lineage(parent).map_err(StreamError::from)?;
        let listed = os::entry_stat(&directory, leaf).map_err(|e| StreamError::Unavailable(code(e)))?;
        let mut file = os::open_regular(&directory, leaf).map_err(|e| StreamError::Unavailable(code(e)))?;
        let before = Identity::of(&file).map_err(|e| StreamError::Unavailable(code(e)))?;
        if before != Identity::from_stat(&listed) {
            return Err(StreamError::Changed);
        }
        if before.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG) || before.links != 1 {
            return Err(StreamError::Unsupported);
        }
        await_pressure(0, before.size, pressure)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut hashed = 0u64;
        loop {
            if hashed % (8 * 1024 * 1024) == 0 {
                await_pressure(hashed, before.size, pressure)?;
            }
            let n = file.read(&mut buffer).map_err(|e| StreamError::Unavailable(code(e)))?;
            if n == 0 { break }
            hashed = hashed.checked_add(n as u64).ok_or(StreamError::Changed)?;
            if hashed > before.size { return Err(StreamError::Changed) }
            hasher.update(&buffer[..n]);
        }
        if hashed != before.size { return Err(StreamError::Changed) }
        let digest = format!("{:x}", hasher.finalize());
        file.seek(SeekFrom::Start(0)).map_err(|e| StreamError::Unavailable(code(e)))?;
        begin(&StreamedSource { identity: before, digest: digest.clone(), bytes: before.size, pages: 0 })
            .map_err(StreamError::Consumer)?;
        let (mut offset, mut pages, mut pending, mut read) = (0u64, 0u64, Vec::new(), 0u64);
        loop {
            let page_bytes = await_pressure(offset, before.size, pressure)?;
            let mut page = vec![0u8; page_bytes];
            let n = file.read(&mut page).map_err(|e| StreamError::Unavailable(code(e)))?;
            if n == 0 { break }
            read = read.checked_add(n as u64).ok_or(StreamError::Changed)?;
            if read > before.size { return Err(StreamError::Changed) }
            pending.extend_from_slice(&page[..n]);
            delivered(&mut pending, &mut offset, &mut pages, &mut each)?;
        }
        if read != before.size || !pending.is_empty() || offset != before.size {
            return Err(StreamError::Changed);
        }
        let fresh = self.lineage(parent).map_err(StreamError::from)?;
        let rebound = os::entry_stat(&fresh.0, leaf).map_err(|e| StreamError::Unavailable(code(e)))?;
        if fresh.1 != lineage || Identity::from_stat(&rebound) != before
            || Identity::of(&file).ok() != Some(before)
        {
            return Err(StreamError::Changed);
        }
        Ok(StreamedSource { identity: before, digest, bytes: read, pages })
    }
}

#[cfg(test)]
mod pressure_tests {
    use super::*;
    #[test]
    fn persistent_pressure_returns_exact_offset_without_discarding_prior_pages() {
        let result = await_pressure_for(4096, 8192, std::time::Duration::ZERO, |offset, _| Err(StreamError::Pressure { offset }));
        assert!(matches!(result, Err(StreamError::Pressure { offset: 4096 })));
    }
    #[test]
    fn retained_progress_retries_after_transient_pressure() {
        let mut calls = 0;
        let page = await_pressure(4096, 8192, |offset, _| {
            assert_eq!(offset, 4096);
            calls += 1;
            if calls < 3 { Err(StreamError::Pressure { offset }) } else { Ok(2048) }
        }).unwrap();
        assert_eq!((calls, page), (3, 2048));
    }
}
