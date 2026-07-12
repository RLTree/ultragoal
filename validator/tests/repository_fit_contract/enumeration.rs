#![cfg(target_vendor = "apple")]

use super::repository_fit::{CanonicalPath, FitErrorId, FitReader, LocalRepository};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct Fixture {
    container: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container = PathBuf::from("/tmp").join(format!(
            "hul-fit-enumeration-{label}-{}-{nonce}",
            std::process::id()
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).unwrap();
        Self { container, root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

fn fill(directory: &Path, count: usize) {
    for index in 0..count {
        fs::write(directory.join(format!("entry-{index:03}")), b"x").unwrap();
    }
}

fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>, FitErrorId> {
    let mut reader = LocalRepository::open(root).unwrap();
    reader
        .read_file(&CanonicalPath::parse(path).unwrap(), 1024)
        .map_err(|failure| failure.id())
}

#[test]
fn local_reader_accepts_a_complete_scan_at_the_entry_limit() {
    let fixture = Fixture::new("at-limit");
    fill(&fixture.root, 253);
    fs::write(fixture.root.join("AGENTS.md"), b"exact").unwrap();
    assert_eq!(
        read(&fixture.root, "AGENTS.md").unwrap(),
        Some(b"exact".to_vec())
    );
}

#[test]
fn local_reader_rejects_one_over_and_repeated_ancestor_scans() {
    let one_over = Fixture::new("one-over");
    fill(&one_over.root, 255);
    assert_eq!(
        read(&one_over.root, "AGENTS.md").unwrap_err(),
        FitErrorId::ResourceLimit
    );

    let repeated = Fixture::new("repeated");
    let mut directory = repeated.root.clone();
    let mut components = Vec::new();
    for depth in 0..15 {
        fill(&directory, 252);
        let component = format!("d{depth:02}");
        fs::create_dir(directory.join(&component)).unwrap();
        directory.push(&component);
        components.push(component);
    }
    components.push("AGENTS.md".into());
    assert_eq!(
        read(&repeated.root, &components.join("/")).unwrap_err(),
        FitErrorId::ResourceLimit
    );
}

#[test]
fn local_reader_hard_stops_while_a_directory_is_churning() {
    let fixture = Fixture::new("churn");
    fill(&fixture.root, 300);
    let left = fixture.root.join("churn-left");
    let right = fixture.root.join("churn-right");
    fs::write(&left, b"churn").unwrap();
    let running = Arc::new(AtomicBool::new(true));
    let worker_running = Arc::clone(&running);
    let worker = std::thread::spawn(move || {
        while worker_running.load(Ordering::Relaxed) {
            if fs::rename(&left, &right).is_ok() {
                let _ = fs::rename(&right, &left);
            }
        }
    });
    let result = read(&fixture.root, "AGENTS.md");
    running.store(false, Ordering::Relaxed);
    worker.join().unwrap();
    assert_eq!(result.unwrap_err(), FitErrorId::ResourceLimit);
}
