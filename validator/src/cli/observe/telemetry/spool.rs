use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write(root: &Path, event: &Value) -> Result<(), String> {
    let dir = ensure_dir(root)?;
    let path = dir.join("events.jsonl");
    let line = serde_json::to_string(event).expect("serde_json::Value serialization is infallible");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| spool_io_error(&path, err))?;
    write_line(&mut file, &line, &path)
}

pub(super) fn ensure_dir(root: &Path) -> Result<std::path::PathBuf, String> {
    let dir = root.join("validation_artifacts/observability/spool");
    fs::create_dir_all(&dir).map_err(|err| spool_io_error(&dir, err))?;
    Ok(dir)
}

fn write_line<W: Write>(writer: &mut W, line: &str, path: &Path) -> Result<(), String> {
    writeln!(writer, "{line}").map_err(|err| spool_io_error(path, err))
}

fn spool_io_error(path: &Path, err: io::Error) -> String {
    format!("{}: {err}", path.display())
}

#[cfg(test)]
pub(crate) fn write_line_read_only_failure_for_test(path: &Path) -> String {
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .expect("read-only spool test file opens");
    write_line(&mut file, "line", path).expect_err("read-only file rejects write")
}
