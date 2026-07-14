fn actor_record(actor: &str, lock_identity: (u64, u64), ledger_identity: (u64, u64)) -> String {
    format!(
        "{actor}\nlock-device:{}\nlock-inode:{}\nledger-device:{}\nledger-inode:{}\n",
        lock_identity.0, lock_identity.1, ledger_identity.0, ledger_identity.1
    )
}

fn recover_partial_initialization(
    directory: &File,
    present: &[bool; 3],
) -> Result<(), ProductError> {
    if present[0] {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    if present[2] {
        let ledger = open_file_at(directory, LEDGER_NAME, libc::O_RDONLY, MAX_LEDGER_BYTES)?;
        if ledger
            .metadata()
            .map_err(|_| ProductError::AuthorityStoreInvalid)?
            .len()
            != 0
        {
            return Err(ProductError::AuthorityStoreInvalid);
        }
    }
    for (is_present, name, max) in [
        (present[0], ACTOR_NAME, 256),
        (present[1], KEY_NAME, 32),
        (present[2], LEDGER_NAME, MAX_LEDGER_BYTES),
    ] {
        if is_present {
            open_file_at(directory, name, libc::O_RDONLY, max)?;
            unlink_file_at(directory, name)?;
        }
    }
    directory
        .sync_all()
        .map_err(|_| ProductError::AuthorityStoreInvalid)
}
