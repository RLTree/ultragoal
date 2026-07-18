use std::fs::{self, File};
use std::io::{Seek, Write};
use std::path::Path;

const FIXED_DOS_TIME: u16 = 0;
const FIXED_DOS_DATE: u16 = 33;

trait WriteSeek: Write + Seek {}

impl<T: Write + Seek> WriteSeek for T {}

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub bytes: Vec<u8>,
    pub crc32: u32,
    pub offset: u32,
}

pub fn write_zip(zip_path: &Path, rows: &mut [Entry]) -> Result<(), String> {
    let (tmp, mut file) = create_temp_file(zip_path)?;
    write_zip_stream(&mut file, rows)?;
    sync_result(&tmp, file.sync_all())?;
    drop(file);
    fs::rename(&tmp, zip_path)
        .map_err(|err| format!("{}: zip rename failed: {err}", zip_path.display()))
}

fn create_temp_file(zip_path: &Path) -> Result<(std::path::PathBuf, File), String> {
    let parent = zip_path
        .parent()
        .ok_or_else(|| format!("{}: zip path lacks parent", zip_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("{}: zip parent create failed: {err}", parent.display()))?;
    let name = zip_path
        .file_name()
        .ok_or_else(|| format!("{}: zip path lacks file name", zip_path.display()))?
        .to_string_lossy();
    let temp = parent.join(format!(".{name}.tmp-{}", std::process::id()));
    let _ = fs::remove_file(&temp);
    let file = File::create(&temp)
        .map_err(|err| format!("{}: zip create failed: {err}", temp.display()))?;
    Ok((temp, file))
}

pub(crate) fn sync_result(path: &Path, result: std::io::Result<()>) -> Result<(), String> {
    result.map_err(|err| format!("{}: zip sync failed: {err}", path.display()))
}

fn write_zip_stream(file: &mut dyn WriteSeek, rows: &mut [Entry]) -> Result<(), String> {
    for row in rows.iter_mut() {
        row.offset = pos_u32(file)?;
        write_local_header(file, row)?;
    }
    let central_offset = pos_u32(file)?;
    for row in rows.iter() {
        write_central_header(file, row)?;
    }
    let central_size = pos_u32(file)? - central_offset;
    write_end_record(file, rows.len() as u16, central_size, central_offset)
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn write_local_header(file: &mut dyn Write, row: &Entry) -> Result<(), String> {
    write_u32(file, 0x04034b50)?;
    write_u16(file, 20)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, FIXED_DOS_TIME)?;
    write_u16(file, FIXED_DOS_DATE)?;
    write_u32(file, row.crc32)?;
    write_u32(file, row.bytes.len() as u32)?;
    write_u32(file, row.bytes.len() as u32)?;
    write_u16(file, row.name.len() as u16)?;
    write_u16(file, 0)?;
    file.write_all(row.name.as_bytes())
        .map_err(|err| format!("zip local name write failed: {err}"))?;
    file.write_all(&row.bytes)
        .map_err(|err| format!("zip local bytes write failed: {err}"))
}

fn write_central_header(file: &mut dyn Write, row: &Entry) -> Result<(), String> {
    write_u32(file, 0x02014b50)?;
    write_u16(file, 0x0314)?;
    write_u16(file, 20)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, FIXED_DOS_TIME)?;
    write_u16(file, FIXED_DOS_DATE)?;
    write_u32(file, row.crc32)?;
    write_u32(file, row.bytes.len() as u32)?;
    write_u32(file, row.bytes.len() as u32)?;
    write_u16(file, row.name.len() as u16)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u32(file, 0o100644 << 16)?;
    write_u32(file, row.offset)?;
    file.write_all(row.name.as_bytes())
        .map_err(|err| format!("zip central name write failed: {err}"))
}

fn write_end_record(
    file: &mut dyn Write,
    count: u16,
    size: u32,
    offset: u32,
) -> Result<(), String> {
    write_u32(file, 0x06054b50)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, count)?;
    write_u16(file, count)?;
    write_u32(file, size)?;
    write_u32(file, offset)?;
    write_u16(file, 0)
}

fn pos_u32(file: &mut dyn Seek) -> Result<u32, String> {
    let pos = file
        .stream_position()
        .map_err(|err| format!("zip seek failed: {err}"))?;
    u32::try_from(pos).map_err(|_| "zip exceeds 32-bit size limit".to_string())
}

pub(crate) fn write_u16(file: &mut dyn Write, value: u16) -> Result<(), String> {
    file.write_all(&value.to_le_bytes())
        .map_err(|err| format!("zip write failed: {err}"))
}

pub(crate) fn write_u32(file: &mut dyn Write, value: u32) -> Result<(), String> {
    file.write_all(&value.to_le_bytes())
        .map_err(|err| format!("zip write failed: {err}"))
}

#[cfg(test)]
mod tests;
