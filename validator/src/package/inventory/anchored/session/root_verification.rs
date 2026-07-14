impl Session {
    fn verify_root_descriptor(&self) -> Result<(), String> {
        let opened = self
            .root_file
            .metadata()
            .map_err(|_| "anchored package root metadata failed".to_string())?;
        if !self.root_matches(Snapshot::from_metadata(&opened)) {
            return Err("anchored package root changed during session".to_string());
        }
        Ok(())
    }
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|_| "anchored package current directory unavailable".to_string())
}

fn validate_regular(snapshot: Snapshot, maximum: u64) -> Result<(), String> {
    if !snapshot.is_regular() {
        return Err("anchored package target is not a regular file".to_string());
    }
    if snapshot.links() != 1 {
        return Err("anchored package target has multiple hard links".to_string());
    }
    if snapshot.size() < 0 || snapshot.size() as u64 > maximum {
        return Err("anchored package file exceeds its byte limit".to_string());
    }
    Ok(())
}

fn compare_opened(file: &File, expected: Snapshot, kind: &str) -> Result<(), String> {
    let actual = file
        .metadata()
        .map(|metadata| Snapshot::from_metadata(&metadata))
        .map_err(|_| format!("anchored package {kind} metadata failed"))?;
    if actual != expected {
        return Err(format!("anchored package {kind} changed during open"));
    }
    Ok(())
}

#[cfg(test)]
fn before_root_hook() {
    super::test_hooks::before_root();
}
#[cfg(not(test))]
fn before_root_hook() {}

#[cfg(test)]
fn before_component_hook(relative: &str, component: usize) {
    super::test_hooks::before_component(relative, component);
}
#[cfg(not(test))]
fn before_component_hook(_relative: &str, _component: usize) {}

#[cfg(test)]
fn after_read_hook(relative: &str) {
    super::test_hooks::after_read(relative);
}
#[cfg(not(test))]
fn after_read_hook(_relative: &str) {}

#[cfg(test)]
fn before_finish_hook() {
    super::test_hooks::before_finish();
}
#[cfg(not(test))]
fn before_finish_hook() {}
