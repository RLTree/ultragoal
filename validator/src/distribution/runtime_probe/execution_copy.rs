struct RuntimeExecutionCopy {
    directory: PathBuf,
    path: PathBuf,
}

impl PinnedRuntimeExecutable {
    fn execution_copy(&self) -> Result<RuntimeExecutionCopy, DistributionError> {
        use std::io::Write;
        use std::os::unix::fs::{FileExt, OpenOptionsExt, PermissionsExt};

        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| error(DistributionErrorId::EffectFailed))?;
        let directory = std::env::temp_dir().join(format!(
            "harness-ultragoal-runtime-{}-{}",
            std::process::id(),
            hex(&nonce)
        ));
        std::fs::create_dir(&directory).map_err(|_| error(DistributionErrorId::EffectFailed))?;
        let result = (|| {
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| error(DistributionErrorId::EffectFailed))?;
            let path = directory.join("ultragoal");
            let mut options = std::fs::OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .mode(0o700);
            let mut output = options
                .open(&path)
                .map_err(|_| error(DistributionErrorId::EffectFailed))?;
            let size = self
                .file
                .metadata()
                .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?
                .len();
            let mut offset = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            while offset < size {
                let limit = (size - offset).min(buffer.len() as u64) as usize;
                let count = self
                    .file
                    .read_at(&mut buffer[..limit], offset)
                    .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
                if count == 0 {
                    return Err(error(DistributionErrorId::ObjectChanged));
                }
                output
                    .write_all(&buffer[..count])
                    .map_err(|_| error(DistributionErrorId::EffectFailed))?;
                offset += count as u64;
            }
            output
                .sync_all()
                .map_err(|_| error(DistributionErrorId::EffectFailed))?;
            let copied =
                std::fs::read(&path).map_err(|_| error(DistributionErrorId::EffectFailed))?;
            if sha256(&copied) != self.sha256 {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            Ok(RuntimeExecutionCopy {
                directory: directory.clone(),
                path,
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&directory);
        }
        result
    }
}

impl RuntimeExecutionCopy {
    fn path(&self) -> &Path {
        &self.path
    }

    fn working_directory(&self) -> &Path {
        &self.directory
    }

    fn remove(self) -> Result<(), DistributionError> {
        std::fs::remove_file(&self.path).map_err(|_| error(DistributionErrorId::EffectFailed))?;
        std::fs::remove_dir(&self.directory).map_err(|_| error(DistributionErrorId::EffectFailed))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
