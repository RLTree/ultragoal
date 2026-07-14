use super::*;

pub(crate) fn emit(outcome: RuntimeOutcome, mode: OutputMode) -> Result<i32, String> {
    let streams = outcome.render(mode);
    write_all(io::stdout().lock(), &streams.stdout)?;
    write_all(io::stderr().lock(), &streams.stderr)?;
    Ok(streams.exit_code)
}

pub(crate) fn emit_text(text: String) -> Result<i32, String> {
    let mut bytes = text.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    write_all(io::stdout().lock(), &bytes)?;
    Ok(0)
}

pub(crate) fn emit_compatibility(text: String) -> Result<i32, String> {
    let mut bytes = text.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    write_all(io::stderr().lock(), &bytes)?;
    Ok(crate::cli::successor::compatibility::COMPATIBILITY_EXIT_CODE)
}

pub(crate) fn write_all(mut output: impl Write, bytes: &[u8]) -> Result<(), String> {
    output
        .write_all(bytes)
        .map_err(|_| "successor runtime output failed".to_owned())
}
