use super::*;

impl HostState {
    pub(crate) fn open(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        if !home.is_absolute()
            || fs::canonicalize(home).map_err(|_| HostFailure::Unavailable)? != home
        {
            return Err(HostFailure::Invalid);
        }
        let home_directory = AnchoredDirectory::open_absolute(home)?;
        let mut current = home_directory.open_child(STATE_COMPONENTS[0])?;
        for component in &STATE_COMPONENTS[1..] {
            current = current.open_child(component)?;
        }
        let authority = current.open_child(AUTHORITY_DIRECTORY)?;
        let adapter = current.open_child(ADAPTER_DIRECTORY)?;
        let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
        let lock_identity = identity(&lock.metadata().map_err(|_| HostFailure::Invalid)?);
        let lock = ProcessLock::acquire(lock)?.0;
        let mut marker_file = lock.try_clone().map_err(|_| HostFailure::Invalid)?;
        marker_file
            .seek(SeekFrom::Start(0))
            .map_err(|_| HostFailure::Invalid)?;
        let mut marker = Vec::new();
        marker_file
            .take((LOCK_MARKER.len() + 1) as u64)
            .read_to_end(&mut marker)
            .map_err(|_| HostFailure::Invalid)?;
        if marker != LOCK_MARKER {
            return Err(HostFailure::Invalid);
        }
        let target_id = target_identity(target)?;
        let state = Self {
            home: home_directory,
            state: current,
            authority,
            adapter,
            lock,
            lock_identity,
            target_id,
        };
        state.verify()?;
        Ok(state)
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        self.home.verify()?;
        self.state.verify()?;
        self.authority.verify()?;
        self.adapter.verify()?;
        let metadata = self.lock.metadata().map_err(|_| HostFailure::Invalid)?;
        if identity(&metadata) != self.lock_identity
            || self.adapter.stat(LOCK_NAME)? != Some(self.lock_identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }

    pub(crate) fn read_reuse(
        &self,
        expected: &CacheBinding,
    ) -> Result<Option<Vec<Vec<u8>>>, HostFailure> {
        self.verify()?;
        let Some(mut file) = self.adapter.open_optional_regular(CACHE_NAME, 0o600)? else {
            return Ok(None);
        };
        let bytes = read_bounded(&mut file, MAX_CACHE_BYTES)?;
        let envelope: CacheEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| HostFailure::Invalid)?;
        let canonical = serde_json::to_vec(&envelope).map_err(|_| HostFailure::Invalid)?;
        if canonical != bytes {
            return Err(HostFailure::Invalid);
        }
        if envelope.schema_version == RETIRED_CACHE_SCHEMA {
            self.verify()?;
            return Ok(None);
        }
        if envelope.schema_version != CACHE_SCHEMA
            || &envelope.binding != expected
            || decode_hex_exact(&envelope.nonce_hex, NONCE_BYTES).is_err()
            || envelope.artifact_sha256.is_empty()
            || envelope.artifact_sha256.len() != envelope.artifacts_hex.len()
            || monotonic_tick()? < envelope.issued_monotonic_tick
        {
            return Err(HostFailure::Invalid);
        }
        let mut total = 0_usize;
        let mut artifacts = Vec::with_capacity(envelope.artifacts_hex.len());
        for (expected_digest, encoded) in
            envelope.artifact_sha256.iter().zip(&envelope.artifacts_hex)
        {
            let artifact = decode_hex(encoded)?;
            total = total
                .checked_add(artifact.len())
                .ok_or(HostFailure::Invalid)?;
            if artifact.is_empty()
                || total > MAX_ARTIFACT_BYTES
                || digest(&artifact) != *expected_digest
            {
                return Err(HostFailure::Invalid);
            }
            artifacts.push(artifact);
        }
        self.verify()?;
        Ok(Some(artifacts))
    }

    pub(crate) fn persist_reuse(
        &self,
        binding: CacheBinding,
        artifacts: &[Vec<u8>],
    ) -> Result<(), HostFailure> {
        if artifacts.is_empty()
            || artifacts.iter().map(Vec::len).sum::<usize>() > MAX_ARTIFACT_BYTES
        {
            return Err(HostFailure::Persistence);
        }
        self.verify()?;
        if self.adapter.stat(CACHE_NAME)?.is_some() {
            self.adapter
                .open_optional_regular(CACHE_NAME, 0o600)?
                .ok_or(HostFailure::Invalid)?;
        }
        let mut nonce = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| HostFailure::RandomUnavailable)?;
        let envelope = CacheEnvelope {
            schema_version: CACHE_SCHEMA.to_owned(),
            binding,
            issued_monotonic_tick: monotonic_tick()?,
            nonce_hex: hex(&nonce),
            artifact_sha256: artifacts.iter().map(|bytes| digest(bytes)).collect(),
            artifacts_hex: artifacts.iter().map(|bytes| hex(bytes)).collect(),
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|_| HostFailure::Persistence)?;
        if bytes.len() as u64 > MAX_CACHE_BYTES {
            return Err(HostFailure::Persistence);
        }
        let temp_name = format!("reuse.{}.tmp", hex(&nonce));
        let mut temp = self.adapter.create_exclusive(&temp_name, 0o600)?;
        let write = (|| {
            temp.write_all(&bytes)
                .map_err(|_| HostFailure::Persistence)?;
            temp.sync_all().map_err(|_| HostFailure::Persistence)?;
            self.adapter.rename(&temp_name, CACHE_NAME)?;
            self.adapter
                .file
                .sync_all()
                .map_err(|_| HostFailure::Persistence)?;
            Ok(())
        })();
        if write.is_err() {
            let _ = self.adapter.unlink(&temp_name);
        }
        write?;
        let observed = self
            .read_reuse(&envelope.binding)?
            .ok_or(HostFailure::Persistence)?;
        if observed != artifacts {
            return Err(HostFailure::Persistence);
        }
        self.verify()
    }
}
