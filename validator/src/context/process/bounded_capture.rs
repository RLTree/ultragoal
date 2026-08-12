use std::io::{self, Read};

pub(super) const MAX_CAPTURE_BYTES: usize = 64 * 1024 * 1024;
const DRAIN_SLICE_BYTES: usize = 256 * 1024;

pub(super) struct Captured {
    pub(super) bytes: Vec<u8>,
    pub(super) overflow: bool,
    pub(super) eof: bool,
}

impl Captured {
    pub(super) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            overflow: false,
            eof: false,
        }
    }

    pub(super) fn drain(&mut self, reader: &mut impl Read) -> io::Result<()> {
        let mut buffer = [0_u8; 16 * 1024];
        let mut drained = 0_usize;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(count) => {
                    drained = drained.saturating_add(count);
                    let retained = MAX_CAPTURE_BYTES
                        .saturating_sub(self.bytes.len())
                        .min(count);
                    self.bytes.extend_from_slice(&buffer[..retained]);
                    self.overflow |= retained != count;
                    if drained >= DRAIN_SLICE_BYTES {
                        return Ok(());
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error),
            }
        }
    }
}
