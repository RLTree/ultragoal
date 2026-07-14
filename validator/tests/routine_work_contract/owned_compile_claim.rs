use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::owned_compile_quarantine::entry_identity;
use super::owned_compile_scratch::{OwnedCompileScratch, configured_root};

const MARKER_NAME: &[u8] = b"OWNERSHIP.v1";
type HmacSha256 = Hmac<Sha256>;

pub(crate) struct ClaimMarker {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) mac: [u8; 32],
}

pub(crate) fn random_claim_name(label: &str) -> CString {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).expect("scratch claim randomness unavailable");
    CString::new(format!("{label}-{}", hex(&nonce))).unwrap()
}

pub(crate) fn create_marker(
    directory: RawFd,
    root_device: u64,
    root_inode: u64,
    name: &CStr,
    device: u64,
    inode: u64,
    secret: &[u8; 32],
) -> io::Result<ClaimMarker> {
    let mac = claim_mac(root_device, root_inode, name, device, inode, secret);
    let marker_name = CString::new(MARKER_NAME).unwrap();
    let fd = unsafe {
        libc::openat(
            directory,
            marker_name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut marker = unsafe { File::from_raw_fd(fd) };
    std::io::Write::write_all(&mut marker, &mac)?;
    let metadata = marker.metadata()?;
    Ok(ClaimMarker {
        device: metadata.dev(),
        inode: metadata.ino(),
        mac,
    })
}

pub(crate) fn authenticates_claim(scratch: &OwnedCompileScratch) -> bool {
    let root = configured_root("CODEX_WORKTREE_SCRATCH");
    let Ok(root_metadata) = scratch.parent.metadata() else {
        return false;
    };
    let Ok(current_root_metadata) = root.metadata() else {
        return false;
    };
    if scratch.path.parent() != Some(root.as_path())
        || (root_metadata.dev(), root_metadata.ino()) != (scratch.root_device, scratch.root_inode)
        || (current_root_metadata.dev(), current_root_metadata.ino())
            != (scratch.root_device, scratch.root_inode)
        || entry_identity(scratch.parent.as_raw_fd(), &scratch.name)
            != Some((scratch.device, scratch.inode))
    {
        return false;
    }
    let expected = claim_mac(
        scratch.root_device,
        scratch.root_inode,
        &scratch.name,
        scratch.device,
        scratch.inode,
        &scratch.secret,
    );
    expected == scratch.marker_mac && marker_matches(scratch, &expected)
}

fn marker_matches(scratch: &OwnedCompileScratch, expected: &[u8; 32]) -> bool {
    let name = CString::new(MARKER_NAME).unwrap();
    let fd = unsafe {
        libc::openat(
            scratch.directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return false;
    }
    let mut marker = unsafe { File::from_raw_fd(fd) };
    let Ok(metadata) = marker.metadata() else {
        return false;
    };
    if (metadata.dev(), metadata.ino()) != (scratch.marker_device, scratch.marker_inode)
        || !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o777 != 0o600
        || metadata.len() != expected.len() as u64
    {
        return false;
    }
    let mut bytes = [0_u8; 32];
    marker.read_exact(&mut bytes).is_ok() && bytes == *expected
}

fn claim_mac(
    root_device: u64,
    root_inode: u64,
    name: &CStr,
    device: u64,
    inode: u64,
    secret: &[u8; 32],
) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(b"RoutineCompileScratchClaim-v1\0");
    for bytes in [
        root_device.to_be_bytes().as_slice(),
        root_inode.to_be_bytes().as_slice(),
        name.to_bytes(),
        device.to_be_bytes().as_slice(),
        inode.to_be_bytes().as_slice(),
    ] {
        mac.update(&(bytes.len() as u64).to_be_bytes());
        mac.update(bytes);
    }
    mac.finalize().into_bytes().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
