use super::*;

pub(crate) fn start_input_writer(
    mut stdin: File,
    bytes: Vec<u8>,
) -> Result<WriterHandle, RoutineError> {
    std::thread::Builder::new()
        .name("routine-mediator-stdin".to_owned())
        .spawn(move || {
            stdin
                .write_all(&bytes)
                .and_then(|_| stdin.flush())
                .map_err(|_| mediator_error("mediator-stdin-write-failed"))
        })
        .map_err(|_| mediator_error("mediator-stdin-writer-start-failed"))
}
