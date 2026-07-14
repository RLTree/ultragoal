pub(super) fn verify(
    reads: &ReadSession,
    repository_root: &Path,
    row: &ContextRow,
) -> Result<Vec<String>, InventoryError> {
    if !registry_valid(std::slice::from_ref(row)) {
        return Err(invalid("worker evidence context registry row is invalid"));
    }
    let historical = declared_historical(row)
        .ok_or_else(|| invalid("historical worker evidence rows are invalid"))?;
    let root = repository_root.join(CONTEXT_ROOT);
    let metadata = fs::symlink_metadata(&root).map_err(|_| {
        invalid("worker evidence directory is unavailable or not a regular directory")
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("worker evidence directory is not confined"));
    }
    #[cfg(unix)]
    {
        let pinned = reads
            .pin_directory(&root)
            .map_err(|_| invalid("worker evidence directory is not descriptor-pinned"))?;
        if pinned != (metadata.dev(), metadata.ino()) {
            return Err(invalid("worker evidence directory identity changed"));
        }
    }
    let entries =
        fs::read_dir(&root).map_err(|_| invalid("worker evidence directory enumeration failed"))?;
    let mut files = BTreeSet::new();
    let mut historical_seen = BTreeSet::new();
    let mut total = 0_u64;
    for entry in entries {
        reads
            .charge_entry()
            .map_err(|_| invalid("worker evidence exceeds the read-session bound"))?;
        let entry = entry.map_err(|_| invalid("worker evidence enumeration failed"))?;
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("worker evidence filename is not UTF-8"))?;
        if !valid_name(&file_name) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|_| invalid("worker evidence file type is unavailable"))?;
        if !file_type.is_file() || file_type.is_symlink() {
            return Err(invalid(
                "worker evidence policy matched a non-regular entry",
            ));
        }
        let bytes = read_bounded(reads, &entry.path(), MAX_FILE_BYTES)?;
        total = total.saturating_add(bytes.len() as u64);
        let record_valid = match historical.get(&file_name) {
            Some(expected) => {
                let exact = sha256_hex(&bytes) == *expected;
                if exact {
                    historical_seen.insert(file_name.clone());
                }
                exact
            }
            None => valid_record(&bytes, &file_name),
        };
        if total > MAX_TOTAL_BYTES || !record_valid {
            return Err(invalid("worker evidence record is malformed or mismatched"));
        }
        let relative = relative(repository_root, &entry.path())?;
        if !files.insert(relative) || files.len() > MAX_FILES {
            return Err(invalid("worker evidence coverage is invalid"));
        }
    }
    if historical_seen != historical.keys().cloned().collect() {
        return Err(invalid("historical worker evidence coverage is incomplete"));
    }
    Ok(files.into_iter().collect())
}

pub(super) const fn context_id() -> &'static str {
    CONTEXT_ID
}

pub(super) const fn schema_ref() -> &'static str {
    SCHEMA_REF
}
