use super::*;

impl Store {
    pub(crate) fn open_or_create_lock(&self) -> Result<File, LedgerError> {
        match self.create_exclusive(LOCK_NAME, 0o600) {
            Ok(file) => {
                file.sync_all().map_err(|_| ledger_io())?;
                self.directory.sync_all().map_err(|_| ledger_io())?;
                Ok(file)
            }
            Err(error) if error.id() == LedgerErrorId::Replay => {
                self.open_existing(LOCK_NAME, libc::O_RDWR)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn create_key(&self) -> Result<File, LedgerError> {
        let mut bytes = [0u8; KEY_BYTES];
        fill(&mut bytes).map_err(|_| ledger_io())?;
        let mut file = self.create_exclusive(KEY_NAME, 0o600)?;
        file.write_all(&bytes).map_err(|_| ledger_io())?;
        file.sync_all().map_err(|_| ledger_io())?;
        self.directory.sync_all().map_err(|_| ledger_io())?;
        Ok(file)
    }
}

pub(crate) fn initial_payload(
    store_id: &str,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<SnapshotPayload, LedgerError> {
    let head_sha256 = digest(
        &serde_json::to_vec(&(
            INITIAL_HEAD_DOMAIN,
            store_id,
            authority_id,
            key_id,
            root_identity,
            lock_identity,
        ))
        .map_err(|_| invalid_transition())?,
    );
    Ok(SnapshotPayload {
        schema_version: LEDGER_SCHEMA.to_owned(),
        store_id: store_id.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: key_id.to_owned(),
        root_identity,
        lock_identity,
        generation: 0,
        head_sha256,
        events: Vec::new(),
    })
}

pub(crate) fn authority_id(store_id: &str, key_id: &str) -> Result<String, LedgerError> {
    serde_json::to_vec(&(AUTHORITY_DOMAIN, store_id, key_id))
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_transition())
}
