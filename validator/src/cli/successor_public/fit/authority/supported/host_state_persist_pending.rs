use super::*;

impl HostState {
    pub(crate) fn persist_pending(
        &self,
        envelope: &PendingEnvelope,
    ) -> Result<FileIdentity, HostFailure> {
        self.verify()?;
        if stat_at(&self.pending.file, &self.pending_name)?.is_some() {
            return Err(HostFailure::Invalid);
        }
        let bytes = envelope.canonical_bytes()?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_PENDING_BYTES {
            return Err(HostFailure::Persistence);
        }
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| HostFailure::Random)?;
        let temporary = format!("{}.tmp-{}", self.pending_name, encode_hex(&random));
        let mut file = openat(
            &self.pending.file,
            &temporary,
            libc::O_WRONLY
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            0o600,
        )?;
        let result = (|| {
            // SAFETY: the descriptor is owned by `file` and the mode is a fixed permission mask.
            if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                return Err(HostFailure::Persistence);
            }
            let created = identity(&file.metadata().map_err(|_| HostFailure::Persistence)?);
            if !created.safe_regular() || created.size != 0 {
                return Err(HostFailure::Invalid);
            }
            file.write_all(&bytes)
                .map_err(|_| HostFailure::Persistence)?;
            file.sync_all().map_err(|_| HostFailure::Persistence)?;
            let written = identity(&file.metadata().map_err(|_| HostFailure::Persistence)?);
            if !created.same_object(written)
                || written.size != bytes.len() as u64
                || stat_at(&self.pending.file, &temporary)? != Some(written)
            {
                return Err(HostFailure::Invalid);
            }
            rename_exclusive(&self.pending.file, &temporary, &self.pending_name)?;
            sync_directory(&self.pending.file)?;
            let published =
                stat_at(&self.pending.file, &self.pending_name)?.ok_or(HostFailure::Persistence)?;
            if published != written || !published.safe_regular() {
                return Err(HostFailure::Invalid);
            }
            Ok(published)
        })();
        if result.is_err() {
            let _ = unlink_at(&self.pending.file, &temporary);
            let _ = sync_directory(&self.pending.file);
        }
        result
    }
    pub(crate) fn remove_pending(&self, expected: FileIdentity) -> Result<(), HostFailure> {
        self.verify()?;
        if stat_at(&self.pending.file, &self.pending_name)? != Some(expected) {
            return Err(HostFailure::Invalid);
        }
        unlink_at(&self.pending.file, &self.pending_name)?;
        sync_directory(&self.pending.file)?;
        if stat_at(&self.pending.file, &self.pending_name)?.is_some() {
            return Err(HostFailure::Persistence);
        }
        self.verify()
    }
}

pub(crate) struct PendingRecord {
    pub(crate) envelope: PendingEnvelope,
    pub(crate) identity: FileIdentity,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PendingEnvelope {
    pub(crate) schema_version: String,
    pub(crate) target_scope_id: String,
    pub(crate) recovery_intent_hex: String,
    pub(crate) nonce_hex: String,
    pub(crate) binding_sha256: String,
}

impl PendingEnvelope {
    pub(crate) fn new(scope: &str, intent: &[u8], nonce: &[u8]) -> Result<Self, HostFailure> {
        if !valid_digest(scope) || nonce.len() != NONCE_BYTES || intent.is_empty() {
            return Err(HostFailure::Invalid);
        }
        let recovery_intent_hex = encode_hex(intent);
        let nonce_hex = encode_hex(nonce);
        let binding_sha256 = envelope_binding(scope, &recovery_intent_hex, &nonce_hex)?;
        Ok(Self {
            schema_version: PENDING_SCHEMA.to_owned(),
            target_scope_id: scope.to_owned(),
            recovery_intent_hex,
            nonce_hex,
            binding_sha256,
        })
    }

    pub(crate) fn canonical_bytes(&self) -> Result<Vec<u8>, HostFailure> {
        serde_json::to_vec(self).map_err(|_| HostFailure::Persistence)
    }

    pub(crate) fn validate(&self, scope: &str, bytes: &[u8]) -> Result<(), HostFailure> {
        if self.schema_version != PENDING_SCHEMA
            || self.target_scope_id != scope
            || !valid_digest(&self.binding_sha256)
            || decode_hex(&self.nonce_hex)?.len() != NONCE_BYTES
            || decode_hex(&self.recovery_intent_hex)?.is_empty()
            || envelope_binding(scope, &self.recovery_intent_hex, &self.nonce_hex)?
                != self.binding_sha256
            || self.canonical_bytes()? != bytes
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}

pub(crate) fn envelope_binding(
    scope: &str,
    intent: &str,
    nonce: &str,
) -> Result<String, HostFailure> {
    serde_json::to_vec(&(ENVELOPE_DOMAIN, scope, intent, nonce))
        .map(|bytes| digest(&bytes))
        .map_err(|_| HostFailure::Persistence)
}

pub(crate) struct AnchoredDirectory {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) identity: FileIdentity,
}
