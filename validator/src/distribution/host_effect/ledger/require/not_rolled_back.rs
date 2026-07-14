fn require_not_rolled_back(
    observed: &ObservedHead,
    payload: &SnapshotPayload,
) -> Result<(), HostEffectLedgerError> {
    if observed.initialized
        && (payload.generation < observed.generation
            || (payload.generation == observed.generation
                && payload.head_sha256 != observed.head_sha256))
    {
        return Err(tampered());
    }
    Ok(())
}

fn state_name(state: HostEffectState) -> &'static str {
    match state {
        HostEffectState::Reserved => "reserved",
        HostEffectState::InFlight => "in-flight",
        HostEffectState::Settled => "settled",
        HostEffectState::Failed => "failed",
        HostEffectState::Ambiguous => "ambiguous",
    }
}

fn parse_state(value: &str) -> Result<HostEffectState, HostEffectLedgerError> {
    match value {
        "reserved" => Ok(HostEffectState::Reserved),
        "in-flight" => Ok(HostEffectState::InFlight),
        "settled" => Ok(HostEffectState::Settled),
        "failed" => Ok(HostEffectState::Failed),
        "ambiguous" => Ok(HostEffectState::Ambiguous),
        _ => Err(tampered()),
    }
}

fn sign(key: &[u8], bytes: &[u8]) -> Result<String, HostEffectLedgerError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| invalid_record())?;
    mac.update(bytes);
    Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
}

fn verify_mac(key: &[u8], bytes: &[u8], expected: &str) -> Result<(), HostEffectLedgerError> {
    let raw = decode_digest(expected)?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| tampered())?;
    mac.update(bytes);
    mac.verify_slice(&raw).map_err(|_| tampered())
}

fn decode_digest(value: &str) -> Result<[u8; 32], HostEffectLedgerError> {
    if !is_digest(value) {
        return Err(tampered());
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value[7..].as_bytes().chunks_exact(2).enumerate() {
        let high = hex(pair[0]).ok_or_else(tampered)?;
        let low = hex(pair[1]).ok_or_else(tampered)?;
        bytes[index] = high << 4 | low;
    }
    Ok(bytes)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn digest_json(value: &impl Serialize) -> Result<String, HostEffectLedgerError> {
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_record())
}

struct LedgerKey([u8; KEY_BYTES]);

impl Drop for LedgerKey {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    hard_links: u64,
    length: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            hard_links: metadata.nlink(),
            length: metadata.len(),
        }
    }

    fn regular(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
    }

    fn directory(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR)
    }

    fn permissions(self) -> u32 {
        self.mode & 0o777
    }

    fn same_directory_anchor(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }
}

struct Store {
    root: PathBuf,
    canonical_root: PathBuf,
    directory: Arc<File>,
    directory_identity: FileIdentity,
}
