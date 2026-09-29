use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

pub const MAX_AUTHORED_LINES: usize = 250;

#[derive(Default)]
pub struct Inventory {
    pub files: BTreeMap<String, Vec<u8>>,
    pub exclusions: Vec<String>,
    pub failures: Vec<String>,
}

pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('*')
        && !path.contains('?')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn excluded(relative: &str) -> Option<&'static str> {
    if relative
        .split('/')
        .any(|p| matches!(p, "target" | "__pycache__" | ".git" | "node_modules"))
    {
        return Some("runtime_or_compiler_output");
    }
    None
}

fn walk(root: &Path, relative: &str) -> Inventory {
    let mut inventory = Inventory::default();
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) => {
            inventory
                .failures
                .push(format!("inventory_read:{relative}:{error}"));
            return inventory;
        }
    };
    if metadata.file_type().is_symlink() {
        inventory
            .failures
            .push(format!("governed_symlink:{relative}"));
    } else if metadata.is_dir() {
        if let Some(class) = excluded(relative) {
            inventory.exclusions.push(format!("{class}:{relative}"));
            return inventory;
        }
        match fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => inventory.merge(walk(
                            root,
                            &format!("{relative}/{}", entry.file_name().to_string_lossy()),
                        )),
                        Err(error) => inventory
                            .failures
                            .push(format!("inventory_read:{relative}:{error}")),
                    }
                }
            }
            Err(error) => inventory
                .failures
                .push(format!("inventory_read:{relative}:{error}")),
        }
    } else if metadata.is_file() {
        match fs::read(&path) {
            Ok(bytes) => {
                inventory.files.insert(relative.into(), bytes);
            }
            Err(error) => inventory
                .failures
                .push(format!("inventory_read:{relative}:{error}")),
        }
    } else {
        inventory
            .failures
            .push(format!("governed_not_regular:{relative}"));
    }
    inventory
}

pub fn collect(root: &Path) -> Inventory {
    let mut inventory = Inventory::default();
    match fs::read_dir(root) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => inventory.merge(walk(root, &entry.file_name().to_string_lossy())),
                    Err(error) => inventory
                        .failures
                        .push(format!("inventory_root_entry:{error}")),
                }
            }
        }
        Err(error) => inventory
            .failures
            .push(format!("inventory_root_read:{error}")),
    }
    for (path, bytes) in &inventory.files {
        if path.ends_with("Cargo.lock") {
            continue;
        } // validated against Cargo metadata separately
        let lines = bytes.iter().filter(|b| **b == b'\n').count()
            + usize::from(!bytes.is_empty() && bytes.last() != Some(&b'\n'));
        if lines > MAX_AUTHORED_LINES {
            inventory.failures.push(format!(
                "authored_line_cap:{path}:{lines}:maximum={MAX_AUTHORED_LINES}"
            ));
        }
        if std::str::from_utf8(bytes).is_err() {
            inventory.failures.push(format!("authored_not_utf8:{path}"));
        }
    }
    inventory
}

impl Inventory {
    fn merge(&mut self, other: Inventory) {
        self.files.extend(other.files);
        self.failures.extend(other.failures);
        self.exclusions.extend(other.exclusions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_cannot_grant_directory_or_wildcard_exceptions() {
        assert!(safe_path("src/plan/mod.rs"));
        for bad in ["", "../src", "/src", "src/*", "src/?", "src/./x"] {
            assert!(!safe_path(bad), "{bad}");
        }
    }
}
