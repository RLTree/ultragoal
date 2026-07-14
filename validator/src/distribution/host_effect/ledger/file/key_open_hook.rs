impl FileHostEffectLedger {
    #[cfg(test)]
    fn install_key_open_hook(&self, hook: KeyOpenHook) -> Result<(), HostEffectLedgerError> {
        let mut slot = self.key_open_hook.lock().map_err(|_| ledger_io())?;
        if slot.is_some() {
            return Err(invalid_record());
        }
        *slot = Some(hook);
        Ok(())
    }

    #[cfg(test)]
    fn pause_after_lock_open(&self) -> Result<(), HostEffectLedgerError> {
        let hook = self.lock_open_hook.lock().map_err(|_| ledger_io())?.take();
        if let Some(hook) = hook {
            hook.reached.wait();
            hook.release.wait();
        }
        process_test_barrier()
    }

    fn after_key_identity(&self) -> Result<(), HostEffectLedgerError> {
        #[cfg(test)]
        {
            let hook = self.key_open_hook.lock().map_err(|_| ledger_io())?.take();
            if let Some(hook) = hook {
                hook.reached.wait();
                hook.release.wait();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
fn process_test_barrier() -> Result<(), HostEffectLedgerError> {
    let Some(root) = std::env::var_os("HUL_LEDGER_CHILD_BARRIER") else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err(invalid_record());
    }
    let root_identity =
        FileIdentity::from_metadata(&fs::symlink_metadata(&root).map_err(|_| ledger_io())?);
    validate_directory(root_identity)?;
    let value = std::env::var("HUL_LEDGER_CHILD_VALUE").map_err(|_| invalid_record())?;
    if value.len() != 1 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_record());
    }
    let ready = root.join(format!("ready-{value}"));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&ready)
        .map_err(|_| ledger_io())?;
    file.sync_all().map_err(|_| ledger_io())?;
    let ready_identity = FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
    validate_regular(ready_identity, 0, 0o600)?;
    let release = root.join("release");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match fs::symlink_metadata(&release) {
            Ok(metadata) => {
                let identity = FileIdentity::from_metadata(&metadata);
                validate_regular(identity, 0, 0o600)?;
                if identity.length != 0 {
                    return Err(tampered());
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if std::time::Instant::now() >= deadline {
                    return Err(ledger_io());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(_) => return Err(ledger_io()),
        }
    }
}
