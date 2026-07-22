use super::spec::CLI_ENTRY_LIMIT;
use crate::distribution::spec::digest;

/// Bytes for the exact CLI build selected by the caller for one source candidate.
///
/// This type deliberately accepts only native executable payloads. Shell scripts,
/// path references, and probe-like text cannot become the installed CLI entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateCliPayload {
    candidate_id: String,
    sha256: String,
    bytes: Vec<u8>,
}

impl CandidateCliPayload {
    pub fn for_candidate(
        candidate_id: &str,
        bytes: Vec<u8>,
    ) -> Result<Self, ProductionPackageError> {
        if !digest(candidate_id)
            || bytes.is_empty()
            || bytes.len() > CLI_ENTRY_LIMIT
            || !is_native_executable(&bytes)
        {
            return Err(failure(ProductionPackageErrorId::MembershipMismatch));
        }
        Ok(Self {
            candidate_id: candidate_id.to_owned(),
            sha256: sha256(&bytes),
            bytes,
        })
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn is_native_executable(bytes: &[u8]) -> bool {
    macho_executable(bytes)
}

fn macho_executable(bytes: &[u8]) -> bool {
    let Some(header) = bytes.get(..32) else {
        return false;
    };
    let magic = &header[..4];
    let little_endian = magic == [0xcf, 0xfa, 0xed, 0xfe];
    let big_endian = magic == [0xfe, 0xed, 0xfa, 0xcf];
    if !little_endian && !big_endian {
        return false;
    }
    let read_u32 = |offset| {
        let value: [u8; 4] = header[offset..offset + 4].try_into().expect("header slice");
        if little_endian {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        }
    };
    let cpu = read_u32(4);
    let file_type = read_u32(12);
    let commands = read_u32(16) as usize;
    let command_bytes = read_u32(20) as usize;
    (cpu == 0x0100_0007 || cpu == 0x0100_000c)
        && file_type == 2
        && commands > 0
        && command_bytes >= commands.saturating_mul(8)
        && command_bytes <= bytes.len().saturating_sub(32)
        && valid_load_commands(bytes, 32, commands, command_bytes, little_endian)
}

fn valid_load_commands(
    bytes: &[u8],
    offset: usize,
    commands: usize,
    command_bytes: usize,
    little_endian: bool,
) -> bool {
    let Some(table) = bytes.get(offset..offset.saturating_add(command_bytes)) else {
        return false;
    };
    let mut cursor = 0_usize;
    let mut has_segment = false;
    let mut has_entry = false;
    for _ in 0..commands {
        let Some(header) = table.get(cursor..cursor.saturating_add(8)) else {
            return false;
        };
        let read_u32 = |start| {
            let value: [u8; 4] = header[start..start + 4].try_into().expect("load command");
            if little_endian {
                u32::from_le_bytes(value)
            } else {
                u32::from_be_bytes(value)
            }
        };
        let command = read_u32(0);
        let size = read_u32(4) as usize;
        if size < 8 || size % 8 != 0 || table.get(cursor..cursor.saturating_add(size)).is_none() {
            return false;
        }
        has_segment |= command == 0x19 && size >= 72;
        has_entry |= command == 0x8000_0028 && size == 24;
        cursor += size;
    }
    cursor == table.len() && has_segment && has_entry
}
