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
    let (tmp, mut file) = crate::output_path::create_temp_file(zip_path, "zip")?;
    write_zip_stream(&mut file, rows)?;
    sync_result(&tmp, file.sync_all())?;
    drop(file);
    crate::output_path::finish_temp_file(&tmp, zip_path, "zip")
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
mod tests {
    use super::{
        Entry, crc32, pos_u32, write_central_header, write_local_header, write_zip_stream,
    };
    use std::io::{Cursor, Error, ErrorKind, Seek, SeekFrom, Write};

    struct FailAfter {
        bytes_left: usize,
    }

    impl Write for FailAfter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.bytes_left == 0 {
                return Err(Error::other("forced write failure"));
            }
            let written = self.bytes_left.min(buf.len());
            self.bytes_left -= written;
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct BadSeek {
        response: Result<u64, ErrorKind>,
    }

    impl Write for BadSeek {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for BadSeek {
        fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
            self.response
                .map_err(|kind| Error::new(kind, "forced seek failure"))
        }
    }

    fn entry() -> Entry {
        let bytes = b"payload".to_vec();
        Entry {
            name: "root/file.txt".to_string(),
            crc32: crc32(&bytes),
            bytes,
            offset: 0,
        }
    }

    #[test]
    fn zip_stream_writes_success_and_injected_failures() {
        let mut rows = vec![entry()];
        let mut ok = Cursor::new(Vec::new());
        write_zip_stream(&mut ok, &mut rows).expect("zip stream writes");
        assert!(ok.into_inner().starts_with(&0x04034b50u32.to_le_bytes()));

        let mut local_name_failure = FailAfter { bytes_left: 30 };
        assert!(
            write_local_header(&mut local_name_failure, &entry())
                .expect_err("local name write fails")
                .contains("zip local name write failed")
        );

        let mut local_bytes_failure = FailAfter {
            bytes_left: 30 + "root/file.txt".len(),
        };
        assert!(
            write_local_header(&mut local_bytes_failure, &entry())
                .expect_err("local bytes write fails")
                .contains("zip local bytes write failed")
        );

        let mut central_name_failure = FailAfter { bytes_left: 46 };
        assert!(
            write_central_header(&mut central_name_failure, &entry())
                .expect_err("central name write fails")
                .contains("zip central name write failed")
        );

        let mut bad_seek = BadSeek {
            response: Err(ErrorKind::Other),
        };
        assert!(
            pos_u32(&mut bad_seek)
                .expect_err("seek fails")
                .contains("zip seek failed")
        );

        let mut huge_seek = BadSeek {
            response: Ok(u64::from(u32::MAX) + 1),
        };
        assert!(
            pos_u32(&mut huge_seek)
                .expect_err("large zip offset fails")
                .contains("zip exceeds 32-bit size limit")
        );

        let mut exhausted_writer = FailAfter { bytes_left: 0 };
        exhausted_writer.flush().expect("flush is inert");
        let mut seek_error_writer = BadSeek { response: Ok(0) };
        assert_eq!(seek_error_writer.write(b"x").expect("writer write"), 1);
        seek_error_writer.flush().expect("writer flush");
    }
}
