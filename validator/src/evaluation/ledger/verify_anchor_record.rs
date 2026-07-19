fn verify_anchor_record(
    record: &AuthenticatedAnchorRecord,
    key: &[u8; 32],
) -> Result<(), EvaluationLedgerError> {
    let expected = authenticate_anchor_record(record.payload.clone(), key)?;
    if record.mac_sha256 != expected.mac_sha256 || record.head_sha256 != expected.head_sha256 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-anchor-authentication-failed",
        ));
    }
    Ok(())
}

struct ScannedAnchorRecord {
    end: u64,
    record: AuthenticatedAnchorRecord,
}

struct AnchorJournalScan {
    records: Vec<ScannedAnchorRecord>,
    complete_length: u64,
    partial_tail: bool,
}

fn scan_anchor_journal(
    bytes: &[u8],
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &EvaluationExecutionBinding,
) -> Result<AnchorJournalScan, EvaluationLedgerError> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    let mut prior_head = sha256(ANCHOR_GENESIS);
    let mut generation = 0_u64;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Ok(AnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let length = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        if length == 0 || length > MAX_ANCHOR_RECORD_BYTES {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-frame-length-invalid",
            ));
        }
        let end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-anchor-frame-overflow"))?;
        if end > bytes.len() {
            return Ok(AnchorJournalScan {
                records,
                complete_length: offset as u64,
                partial_tail: true,
            });
        }
        let record: AuthenticatedAnchorRecord = serde_json::from_slice(&bytes[offset + 8..end])
            .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-record-malformed"))?;
        verify_anchor_record(&record, key)?;
        if record.payload.schema_version != "EvaluationExecutionAnchorRecord-v1"
            || record.payload.prior_anchor_head_sha256 != prior_head
            || record.payload.core.generation != generation
            || record.payload.core.key_id != key_id
            || record.payload.core.lock_identity != lock_identity
            || record.payload.core.anchor_authority != anchor_authority
            || record.payload.core.binding != *binding
            || !reservation_identity_valid(
                &record.payload.core.state,
                record.payload.core.reservation_id_sha256.as_deref(),
            )
            || !state_valid(&record.payload.core.state)
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-record-binding-invalid",
            ));
        }
        prior_head = record.head_sha256.clone();
        generation = generation
            .checked_add(1)
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-ledger-generation-overflow"))?;
        records.push(ScannedAnchorRecord {
            end: end as u64,
            record,
        });
        offset = end;
    }
    Ok(AnchorJournalScan {
        records,
        complete_length: offset as u64,
        partial_tail: false,
    })
}

fn state_valid(state: &EvaluationLedgerState) -> bool {
    match state {
        EvaluationLedgerState::Published {
            run_sha256,
            artifact_set_sha256,
        }
        | EvaluationLedgerState::Terminal {
            run_sha256,
            artifact_set_sha256,
        } => super::valid_sha256(run_sha256) && super::valid_sha256(artifact_set_sha256),
        EvaluationLedgerState::Interrupted { causal_code }
        | EvaluationLedgerState::RecoveryRequired { causal_code } => {
            super::valid_identifier(causal_code)
        }
        EvaluationLedgerState::Initialized | EvaluationLedgerState::Reserved => true,
    }
}

fn reservation_identity_valid(state: &EvaluationLedgerState, value: Option<&str>) -> bool {
    match state {
        EvaluationLedgerState::Initialized => value.is_none(),
        _ => value.is_some_and(super::valid_sha256),
    }
}

pub(super) fn open_safe_directory(
    path: &Path,
) -> Result<(File, FileIdentity), EvaluationLedgerError> {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
    }
    let root_name = CString::new("/").expect("root path has no NUL");
    let descriptor = unsafe {
        libc::open(
            root_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-root-open-failed",
        ));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    for component in components {
        let Component::Normal(component) = component else {
            return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
        };
        let name = CString::new(component.as_bytes())
            .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-root-unsafe"))?;
        let descriptor = unsafe {
            libc::openat(
                file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0,
            )
        };
        if descriptor < 0 {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-root-open-failed",
            ));
        }
        file = unsafe { File::from_raw_fd(descriptor) };
    }
    let identity = safe_file_identity(&file)?;
    if identity.mode & (libc::S_IFMT as u32) != libc::S_IFDIR as u32 || identity.mode & 0o022 != 0 {
        return Err(EvaluationLedgerError::new("evaluation-ledger-root-unsafe"));
    }
    Ok((file, identity))
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn directory_errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
unsafe fn directory_errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}
