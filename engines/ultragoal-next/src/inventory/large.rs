use super::*;

impl Session {
    /// A selected large file is reassembled only under measured host headroom.
    /// Context selection needs current exact spans, but keeps only excerpts after
    /// this callback; a failed stream becomes a named unavailable input.
    pub(super) fn capture_large(&self, raw: &[u8], observed: &mut Observation) -> Option<(String, Vec<u8>)> {
        let Ok(path) = std::str::from_utf8(raw) else {
            observed.complete = false;
            observed.issues.push((String::new(), "non-UTF8 selected path".into()));
            return None;
        };
        let mut bytes = Vec::new();
        let captured = self.stream_source(path, |_| Ok(()), |_, page| {
            bytes.try_reserve(page.len()).map_err(|_| "source reassembly needs more current headroom".to_string())?;
            bytes.extend_from_slice(page.as_bytes());
            Ok(())
        });
        match captured {
            Ok(source) if source.bytes == bytes.len() as u64 && hash(&bytes) == source.digest => observed.admit(
                Capture { path: raw.to_vec(), bytes: Some(bytes), identity: Some(source.identity), problem: None },
                usize::MAX,
            ),
            Ok(_) => {
                observed.problems.insert(path.into(), ("file".into(), "logical source changed during capture".into()));
                None
            }
            Err(error) => {
                observed.problems.insert(path.into(), ("file".into(), format!("logical source unavailable: {error:?}")));
                None
            }
        }
    }
}
