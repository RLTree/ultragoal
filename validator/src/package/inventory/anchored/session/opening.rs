impl Session {
    pub(crate) fn open(root: &Path) -> Result<Self, String> {
        let root = absolute(root)?;
        let before = fs::symlink_metadata(&root)
            .map_err(|_| "anchored package root metadata failed".to_string())?;
        if before.file_type().is_symlink() || !before.is_dir() {
            return Err("anchored package root is not a regular directory".to_string());
        }
        let root_snapshot = Snapshot::from_metadata(&before);
        before_root_hook();
        let root_file = sys::open_root(&root)?;
        let opened = Snapshot::from_metadata(
            &root_file
                .metadata()
                .map_err(|_| "anchored package root metadata failed".to_string())?,
        );
        if opened != root_snapshot || !opened.is_directory() {
            return Err("anchored package root changed during open".to_string());
        }
        Ok(Self {
            root,
            root_file,
            root_snapshot,
            strict_root_snapshot: None,
            observed: BTreeMap::new(),
            entries: 0,
            bytes: 0,
        })
    }

    pub(crate) fn read(&mut self, relative: &str, maximum: u64) -> Result<Vec<u8>, String> {
        self.read_with_mode(relative, maximum)
            .map(|(bytes, _)| bytes)
    }

    pub(crate) fn read_with_mode(
        &mut self,
        relative: &str,
        maximum: u64,
    ) -> Result<(Vec<u8>, u32), String> {
        if maximum == 0 || maximum > MAX_FILE_BYTES {
            return Err("anchored package file limit is invalid".to_string());
        }
        self.entries = self.entries.saturating_add(1);
        if self.entries > MAX_ENTRIES {
            return Err("anchored package session exceeds its entry limit".to_string());
        }
        let components = sys::components(relative)?;
        let mut parent = self
            .root_file
            .try_clone()
            .map_err(|_| "anchored package root clone failed".to_string())?;
        let mut cursor = PathBuf::new();

        for (index, name) in components.iter().enumerate() {
            cursor.push(name);
            let baseline = sys::nofollow_snapshot(&parent, name)?;
            self.observe(&cursor, baseline)?;
            before_component_hook(relative, index);
            let leaf = index + 1 == components.len();
            if !leaf {
                if !baseline.is_directory() {
                    return Err("anchored package ancestor is not a directory".to_string());
                }
                let opened = sys::open_directory(&parent, name)?;
                compare_opened(&opened, baseline, "directory")?;
                parent = opened;
                continue;
            }
            return self.read_leaf(relative, &parent, name, baseline, maximum);
        }
        Err("anchored package path is empty".to_string())
    }

    pub(crate) fn finish(&self) -> Result<(), String> {
        before_finish_hook();
        self.verify_root()?;
        for (relative, expected) in &self.observed {
            self.verify_path(relative, *expected)?;
        }
        self.verify_root()
    }

    #[cfg(test)]
    pub(crate) fn set_usage_for_test(&mut self, entries: u64, bytes: u64) {
        self.entries = entries;
        self.bytes = bytes;
    }

    fn read_leaf(
        &mut self,
        relative: &str,
        parent: &File,
        name: &std::ffi::OsStr,
        baseline: Snapshot,
        maximum: u64,
    ) -> Result<(Vec<u8>, u32), String> {
        validate_regular(baseline, maximum)?;
        if self
            .bytes
            .saturating_add(u64::try_from(baseline.size()).unwrap_or(u64::MAX))
            > MAX_TOTAL_BYTES
        {
            return Err("anchored package session exceeds its byte limit".to_string());
        }
        let mut file = sys::open_file(parent, name)?;
        compare_opened(&file, baseline, "file")?;
        let mut bytes = Vec::with_capacity(usize::try_from(baseline.size()).unwrap_or(0));
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let remaining = maximum.saturating_add(1).saturating_sub(bytes.len() as u64);
            if remaining == 0 {
                break;
            }
            let capacity = buffer.len().min(remaining as usize);
            let count = file
                .read(&mut buffer[..capacity])
                .map_err(|_| "anchored package file read failed".to_string())?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if bytes.len() as u64 > maximum {
                return Err("anchored package file exceeds its byte limit".to_string());
            }
        }
        compare_opened(&file, baseline, "file")?;
        after_read_hook(relative);
        if sys::nofollow_snapshot(parent, name)? != baseline {
            return Err("anchored package file changed during read".to_string());
        }
        compare_opened(&file, baseline, "file")?;
        self.bytes = self.bytes.saturating_add(bytes.len() as u64);
        if self.bytes > MAX_TOTAL_BYTES {
            return Err("anchored package session exceeds its byte limit".to_string());
        }
        self.verify_root_descriptor()?;
        Ok((bytes, baseline.unix_mode()))
    }

    fn observe(&mut self, relative: &Path, snapshot: Snapshot) -> Result<(), String> {
        if let Some(expected) = self.observed.get(relative) {
            if expected != &snapshot {
                return Err("anchored package object changed between reads".to_string());
            }
        } else {
            self.observed.insert(relative.to_path_buf(), snapshot);
        }
        Ok(())
    }

    fn verify_path(&self, relative: &Path, expected_leaf: Snapshot) -> Result<(), String> {
        let text = relative
            .to_str()
            .ok_or_else(|| "anchored package path is not UTF-8".to_string())?;
        let components = sys::components(text)?;
        let mut parent = self
            .root_file
            .try_clone()
            .map_err(|_| "anchored package root clone failed".to_string())?;
        let mut cursor = PathBuf::new();
        for (index, name) in components.iter().enumerate() {
            cursor.push(name);
            let expected = self.observed.get(&cursor).copied().unwrap_or(expected_leaf);
            if sys::nofollow_snapshot(&parent, name)? != expected {
                return Err("anchored package object changed before finalization".to_string());
            }
            let leaf = index + 1 == components.len();
            if leaf && !expected.is_directory() && !expected.is_regular() {
                continue;
            }
            let opened = if leaf && expected.is_regular() {
                sys::open_file(&parent, name)?
            } else {
                sys::open_directory(&parent, name)?
            };
            compare_opened(&opened, expected, "object")?;
            parent = opened;
        }
        Ok(())
    }

    fn verify_root(&self) -> Result<(), String> {
        let path = fs::symlink_metadata(&self.root)
            .map_err(|_| "anchored package root final metadata failed".to_string())?;
        if path.file_type().is_symlink() || !self.root_matches(Snapshot::from_metadata(&path)) {
            return Err("anchored package root changed before finalization".to_string());
        }
        self.verify_root_descriptor()
    }
}
