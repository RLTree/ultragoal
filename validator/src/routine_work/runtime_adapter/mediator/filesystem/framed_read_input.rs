use super::*;
use crate::routine_work::{RustSourceFrameInput, encode_rust_source_syntax_frame};

impl ReadConfinement {
    pub(crate) fn rust_source_syntax_frame(
        &self,
        root: &RootAnchor,
    ) -> Result<Vec<u8>, RoutineError> {
        self.validate(root)?;
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let mut owned = Vec::with_capacity(self.sources.len());
            for source in &self.sources {
                let mut reader = source
                    .file
                    .try_clone()
                    .map_err(|_| mediator_error("mediator-read-source-open-failed"))?;
                reader
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
                let limit = source
                    .record
                    .byte_length
                    .checked_add(1)
                    .ok_or_else(|| mediator_error("mediator-read-source-budget-exceeded"))?;
                let mut bytes = Vec::new();
                reader
                    .take(limit)
                    .read_to_end(&mut bytes)
                    .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
                if bytes.len() as u64 != source.record.byte_length {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-length-changed",
                        None,
                    ));
                }
                owned.push((
                    source.record.relative_path.as_str().to_owned(),
                    source.record.sha256.clone(),
                    source.record.byte_length,
                    bytes,
                ));
            }
            let inputs = owned
                .iter()
                .map(|(path, digest, length, bytes)| {
                    RustSourceFrameInput::new(path, digest, *length, bytes)
                })
                .collect::<Vec<_>>();
            let frame = encode_rust_source_syntax_frame(&inputs)
                .map_err(|_| mediator_error("mediator-rust-source-frame-invalid"))?;
            self.validate(root)?;
            Ok(frame)
        }
    }
}
