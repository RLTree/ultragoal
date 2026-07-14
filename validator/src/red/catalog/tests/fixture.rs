use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct CatalogRoot {
    path: PathBuf,
}

impl CatalogRoot {
    pub(super) fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::current_dir()
            .expect("current directory")
            .join(".git/codex-test")
            .join(format!("red-catalog-{label}-{stamp}"));
        fs::create_dir_all(path.join("fixtures/red")).expect("red directory");
        fs::create_dir_all(path.join("templates")).expect("template directory");
        Self { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("parent");
        fs::write(path, bytes).expect("fixture write");
    }

    pub(super) fn packet(&self, file_id: &str, packet_id: &str) {
        self.write(&format!("fixtures/red/{file_id}.json"), packet(packet_id));
    }
}

impl Drop for CatalogRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(super) fn packet(id: &str) -> String {
    format!(
        "{{\n  \"schema\": \"harness-ultragoal.red-packet.v1\",\n  \"id\": \"{id}\",\n  \"expected_failure\": {{\"check_id\": \"fixture-check\", \"error\": \"fixture failure\"}},\n  \"base_fixture_path\": \"fixtures/valid/base.json\",\n  \"json_patch\": [{{\"op\": \"replace\", \"path\": \"/status\", \"value\": \"bad\"}}],\n  \"materialization\": {{\"expected_validation_layer\": \"semantic\", \"first_failure_must_match_expected\": true, \"post_patch_schema_valid\": true}},\n  \"preconditions\": [{{\"exists\": true, \"path\": \"/status\"}}],\n  \"postconditions\": [{{\"expectation\": \"fixture failure\", \"path\": \"/status\"}}],\n  \"notes\": \"fixture\"\n}}\n"
    )
}
