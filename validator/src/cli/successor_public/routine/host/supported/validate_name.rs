use super::*;

pub(crate) fn validate_name(name: &str) -> Result<(), HostFailure> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.as_bytes().contains(&b'/')
        || name.as_bytes().contains(&0)
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

pub(crate) fn identity(metadata: &fs::Metadata) -> Identity {
    Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
        links: metadata.nlink(),
    }
}

pub(crate) fn stat_identity(metadata: &libc::stat) -> Identity {
    Identity {
        device: metadata.st_dev as u64,
        inode: metadata.st_ino as u64,
        owner: metadata.st_uid,
        mode: metadata.st_mode as u32,
        links: metadata.st_nlink as u64,
    }
}

pub(crate) fn same_anchored_directory(left: Identity, right: Identity) -> bool {
    left.device == right.device
        && left.inode == right.inode
        && left.owner == right.owner
        && left.mode == right.mode
}

pub(crate) fn target_identity(target: &Path) -> Result<String, HostFailure> {
    let canonical = fs::canonicalize(target).map_err(|_| HostFailure::Invalid)?;
    if canonical != target || target.to_str().is_none() {
        return Err(HostFailure::Invalid);
    }
    let metadata = fs::metadata(target).map_err(|_| HostFailure::Invalid)?;
    #[derive(Serialize)]
    struct Target<'a> {
        domain: &'static str,
        path: &'a str,
        device: u64,
        inode: u64,
        owner: u32,
    }
    let bytes = serde_json::to_vec(&Target {
        domain: "routine-public-target-v1",
        path: target.to_str().ok_or(HostFailure::Invalid)?,
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
    })
    .map_err(|_| HostFailure::Invalid)?;
    Ok(digest(&bytes))
}

pub(crate) fn monotonic_tick() -> Result<u64, HostFailure> {
    let mut value = MaybeUninit::<libc::timespec>::uninit();
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, value.as_mut_ptr()) } != 0 {
        return Err(HostFailure::ClockUnavailable);
    }
    let value = unsafe { value.assume_init() };
    if value.tv_sec < 0 || !(0..1_000_000_000).contains(&value.tv_nsec) {
        return Err(HostFailure::ClockUnavailable);
    }
    let seconds = u64::try_from(value.tv_sec).map_err(|_| HostFailure::ClockUnavailable)?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|tick| tick.checked_add(value.tv_nsec as u64))
        .ok_or(HostFailure::ClockUnavailable)
}

pub(crate) fn read_bounded(file: &mut File, maximum: u64) -> Result<Vec<u8>, HostFailure> {
    let metadata = file.metadata().map_err(|_| HostFailure::Invalid)?;
    if metadata.len() == 0 || metadata.len() > maximum {
        return Err(HostFailure::Invalid);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| HostFailure::Invalid)?;
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| HostFailure::Invalid)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(HostFailure::Invalid);
    }
    Ok(bytes)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(ALPHABET[(byte >> 4) as usize] as char);
        output.push(ALPHABET[(byte & 0x0f) as usize] as char);
    }
    output
}

pub(crate) fn decode_hex_exact(value: &str, expected: usize) -> Result<Vec<u8>, HostFailure> {
    let bytes = decode_hex(value)?;
    if bytes.len() != expected {
        return Err(HostFailure::Invalid);
    }
    Ok(bytes)
}

pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>, HostFailure> {
    if value.len() % 2 != 0 || value.len() > MAX_ARTIFACT_BYTES.saturating_mul(2) {
        return Err(HostFailure::Invalid);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = nibble(pair[0]).ok_or(HostFailure::Invalid)?;
            let low = nibble(pair[1]).ok_or(HostFailure::Invalid)?;
            Ok((high << 4) | low)
        })
        .collect()
}

pub(crate) fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
