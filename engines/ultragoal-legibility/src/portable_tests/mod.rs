mod receiving;
mod workspace;

use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);
impl Project {
    fn clean() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ej-receiving-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/receiving"),
            &root,
        );
        Self(root)
    }
    fn write(&self, path: &str, value: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }
    fn claim(&self, path: &str, class: &str) {
        let file = self.0.join("docs/legibility/sources.json");
        let mut rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        rows.push(json!({"path":path,"class":class,"owner":"Cargo.toml","purpose":"Explicit test mutation"}));
        std::fs::write(file, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
    fn audit(&self) -> Value {
        crate::run(&self.0, "docs/legibility/registry.json", true)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}
fn has(audit: &Value, prefix: &str) -> bool {
    audit["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure.as_str().unwrap().starts_with(prefix))
}
