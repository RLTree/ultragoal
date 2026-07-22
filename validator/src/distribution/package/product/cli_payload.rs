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
    elf_executable(bytes) || macho_executable(bytes) || pe_executable(bytes)
}

fn elf_executable(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(..20),
        Some([
            0x7f,
            b'E',
            b'L',
            b'F',
            2,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            2 | 3,
            0,
            0xb7 | 0x3e,
            0
        ])
    )
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
        && (file_type == 2 || file_type == 6)
        && commands > 0
        && command_bytes <= bytes.len().saturating_sub(32)
}

fn pe_executable(bytes: &[u8]) -> bool {
    let Some(dos) = bytes.get(..64) else {
        return false;
    };
    if dos[..2] != *b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().expect("DOS offset")) as usize;
    matches!(
        bytes.get(offset..offset + 6),
        Some([b'P', b'E', 0, 0, 0x64 | 0xaa, 0x86 | 0x64])
    )
}
