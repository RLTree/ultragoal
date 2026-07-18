use super::{Entry, crc32, pos_u32, write_central_header, write_local_header, write_zip_stream};
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
