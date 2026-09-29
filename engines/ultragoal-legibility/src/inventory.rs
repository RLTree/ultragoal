use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

pub const MAX_AUTHORED_LINES: usize = 250;

#[derive(Default)]
pub struct Inventory {
    pub files: BTreeMap<String, Vec<u8>>,
    pub exclusions: Vec<String>,
    pub failures: Vec<String>,
}

struct Budget {
    remaining: u64,
    per_file: u64,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn host_page_size(host: libc::host_t, page: *mut libc::vm_size_t) -> libc::kern_return_t;
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn headroom() -> Option<(u64, u64)> {
    let host = unsafe { libc::mach_host_self() };
    if host == 0 {
        return None;
    }
    let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    let mut page: libc::vm_size_t = 0;
    let observed = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut stats as *mut libc::vm_statistics64).cast(),
            &mut count,
        )
    };
    let sized = unsafe { host_page_size(host, &mut page) };
    let closed = unsafe { mach_port_deallocate(libc::mach_task_self(), host) };
    if observed != libc::KERN_SUCCESS
        || sized != libc::KERN_SUCCESS
        || closed != libc::KERN_SUCCESS
        || count < libc::HOST_VM_INFO64_COUNT
        || page == 0
    {
        return None;
    }
    let page = page as u64;
    let available = (stats.free_count as u64)
        .saturating_add(stats.inactive_count as u64)
        .saturating_add(stats.speculative_count as u64)
        .saturating_mul(page);
    let physical = available.saturating_add(
        (stats.active_count as u64)
            .saturating_add(stats.wire_count as u64)
            .saturating_add(stats.compressor_page_count as u64)
            .saturating_mul(page),
    );
    Some((available, physical))
}

#[cfg(target_os = "linux")]
fn headroom() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let kib = |name: &str| -> Option<u64> {
        text.lines()
            .find_map(|line| line.strip_prefix(name))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some((
        kib("MemAvailable:")?.saturating_mul(1024),
        kib("MemTotal:")?.saturating_mul(1024),
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn headroom() -> Option<(u64, u64)> {
    None
}

impl Budget {
    fn current() -> Option<Self> {
        let (available, physical) = headroom()?;
        let work = available.saturating_sub(physical / 8);
        (work > 0).then_some(Self {
            remaining: work / 8,
            per_file: work / 16,
        })
    }
    fn admits(&self, path: &str, bytes: u64) -> bool {
        bytes <= self.per_file
            && bytes.saturating_add(path.len() as u64).saturating_add(512) <= self.remaining
    }
    fn charge(&mut self, path: &str, bytes: u64) {
        self.remaining = self
            .remaining
            .saturating_sub(bytes.saturating_add(path.len() as u64).saturating_add(512));
    }
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

fn walk(root: &Path, relative: &str, budget: &mut Budget) -> Inventory {
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
                            budget,
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
        if !budget.admits(relative, metadata.len()) {
            inventory.failures.push(format!(
                "inventory_resource_pressure:{relative}:bytes={}:remaining={}",
                metadata.len(),
                budget.remaining
            ));
            return inventory;
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path);
        match file {
            Ok(mut file) => {
                let current = file.metadata();
                match current {
                    Ok(current) if current.is_file() && budget.admits(relative, current.len()) => {
                        let mut bytes = Vec::new();
                        match file
                            .by_ref()
                            .take(budget.per_file.saturating_add(1))
                            .read_to_end(&mut bytes)
                        {
                            Ok(_) if bytes.len() as u64 == current.len() => {
                                budget.charge(relative, bytes.len() as u64);
                                inventory.files.insert(relative.into(), bytes);
                            }
                            Ok(_) => inventory
                                .failures
                                .push(format!("inventory_source_changed:{relative}")),
                            Err(error) => inventory
                                .failures
                                .push(format!("inventory_read:{relative}:{error}")),
                        }
                    }
                    Ok(_) => inventory
                        .failures
                        .push(format!("governed_not_regular:{relative}")),
                    Err(error) => inventory
                        .failures
                        .push(format!("inventory_read:{relative}:{error}")),
                }
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
    let Some(mut budget) = Budget::current() else {
        inventory
            .failures
            .push("inventory_headroom_unavailable".into());
        return inventory;
    };
    match fs::read_dir(root) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => inventory.merge(walk(
                        root,
                        &entry.file_name().to_string_lossy(),
                        &mut budget,
                    )),
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

    #[test]
    fn adaptive_budget_names_oversized_work_without_reading_it() {
        let mut budget = Budget {
            remaining: 1000,
            per_file: 700,
        };
        assert!(budget.admits("small.rs", 100));
        assert!(!budget.admits("large.rs", 701));
        budget.charge("small.rs", 100);
        assert!(!budget.admits("next.rs", 400));
        let root =
            std::env::temp_dir().join(format!("ug-legibility-sparse-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let file = root.join("sparse.rs");
        fs::File::create(&file).unwrap().set_len(1 << 40).unwrap();
        let observed = collect(&root);
        assert!(observed.files.is_empty());
        assert!(
            observed
                .failures
                .iter()
                .any(|f| f.starts_with("inventory_resource_pressure:sparse.rs:")),
            "{:?}",
            observed.failures
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fifo_is_named_without_waiting_for_a_writer() {
        let root = std::env::temp_dir().join(format!("ug-legibility-fifo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let fifo = root.join("registry.json");
        let cpath = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        let start = std::time::Instant::now();
        let observed = collect(&root);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        assert!(
            observed
                .failures
                .contains(&"governed_not_regular:registry.json".into())
        );
        assert!(!observed.files.contains_key("registry.json"));
        fs::remove_dir_all(root).unwrap();
    }
}
