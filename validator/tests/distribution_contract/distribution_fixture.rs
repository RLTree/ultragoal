use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::distribution::{Capability, Layer};

pub const CONTEXT_ID: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const CANDIDATE_ID: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub const PLUGIN_ID: &str = "harness-ultragoal";
pub const VERSION: &str = "0.0.11";

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub root: PathBuf,
    pub request: Value,
}

impl Fixture {
    pub fn complete(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-distribution-{label}-{}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
            unique_time(),
        ));
        fs::create_dir_all(root.join("identity")).expect("identity directory");
        fs::create_dir_all(root.join("payload")).expect("payload directory");
        let payload = b"canonical package byte inventory v1\n";
        let artifact_sha256 = digest(payload);
        for layer in [
            Layer::SourcePackageInput,
            Layer::InstalledBytes,
            Layer::CacheBytes,
        ] {
            fs::write(root.join(payload_path(layer)), payload).expect("payload fixture");
        }
        for layer in Layer::ALL {
            let artifact =
                (layer != Layer::SourcePluginMetadata).then_some(artifact_sha256.as_str());
            fs::write(
                root.join(identity_path(layer)),
                serde_json::to_vec(&envelope(layer, artifact, "affirmed")).expect("identity JSON"),
            )
            .expect("identity fixture");
        }
        Self {
            root,
            request: request(),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.request).expect("request JSON")
    }

    pub fn layer_row_mut(&mut self, layer: Layer) -> &mut Value {
        self.request["layers"]
            .as_array_mut()
            .expect("layer rows")
            .iter_mut()
            .find(|row| row["layer"] == layer.as_str())
            .expect("layer row")
    }

    pub fn capability_row_mut(&mut self, capability: Capability) -> &mut Value {
        self.request["host"]["capabilities"]
            .as_array_mut()
            .expect("capability rows")
            .iter_mut()
            .find(|row| row["capability"] == capability.as_str())
            .expect("capability row")
    }

    pub fn mutate_envelope(&self, layer: Layer, edit: impl FnOnce(&mut Value)) {
        let path = self.root.join(identity_path(layer));
        let mut value: Value = serde_json::from_slice(&fs::read(&path).expect("read envelope"))
            .expect("parse envelope");
        edit(&mut value);
        fs::write(path, serde_json::to_vec(&value).expect("envelope JSON"))
            .expect("write envelope");
    }
}

fn unique_time() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn request() -> Value {
    json!({
        "schema": "harness-ultragoal.distribution-request.v1",
        "context_id": CONTEXT_ID,
        "candidate_id": CANDIDATE_ID,
        "host": {
            "platform": current_platform(),
            "version": "fixture-host-1",
            "capabilities": Capability::ALL.map(|capability| json!({
                "capability": capability.as_str(),
                "state": "supported"
            }))
        },
        "layers": Layer::ALL.map(observed_row)
    })
}

pub fn envelope(layer: Layer, artifact: Option<&str>, exposure: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.distribution-observation.v1",
        "layer": layer.as_str(),
        "context_id": CONTEXT_ID,
        "candidate_id": CANDIDATE_ID,
        "plugin_id": PLUGIN_ID,
        "version": VERSION,
        "artifact_sha256": artifact,
        "exposure": if layer.requires_exposure() { exposure } else { "not-applicable" }
    })
}

pub fn observed_row(layer: Layer) -> Value {
    let mut row = json!({
        "layer": layer.as_str(),
        "state": "observed",
        "identity_path": identity_path(layer)
    });
    if layer.requires_payload() {
        row["payload_path"] = json!(payload_path(layer));
    }
    row
}

pub fn identity_path(layer: Layer) -> String {
    format!("identity/{}.json", layer.as_str())
}

pub fn payload_path(layer: Layer) -> String {
    format!("payload/{}.bin", layer.as_str())
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .expect("read fixture tree")
            .map(|entry| entry.expect("fixture entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).expect("fixture metadata");
            if metadata.is_dir() {
                rows.push((format!("directory:{relative}"), Vec::new()));
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    format!("symlink:{relative}"),
                    fs::read_link(&path)
                        .expect("symlink target")
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((
                    format!("file:{relative}"),
                    fs::read(&path).expect("file bytes"),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

pub fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    }
}
