use crate::distribution::{
    ConfinedRoot, PackagePlan, PackageSnapshot, ScopedFile, build_package,
    plan_package_from_inventory,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, PLUGIN_ID, VERSION, digest, tree};
use crate::package_manifest::manifest;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const CATALOG_ID: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct JourneyFixture {
    pub root: PathBuf,
    pub project: PathBuf,
    pub source: PathBuf,
    pub inventory: Vec<u8>,
}

impl JourneyFixture {
    pub fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-distribution-{label}-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            unique_time(),
        ));
        let project = root.join("project");
        let source = root.join("source");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(source.join(".codex-plugin")).unwrap();
        fs::create_dir_all(source.join("runtime")).unwrap();
        fs::create_dir_all(source.join("skills/harness-ultragoal")).unwrap();
        fs::create_dir_all(source.join("skills/prove")).unwrap();
        let files: [(&str, Vec<u8>, &str, u32); 4] = [
            (
                ".codex-plugin/plugin.json",
                serde_json::to_vec(&manifest()).unwrap(),
                "manifest",
                0o644,
            ),
            (
                "runtime/ultragoal",
                runtime_probe_bytes(),
                "executable",
                0o755,
            ),
            (
                "skills/harness-ultragoal/SKILL.md",
                b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n".to_vec(),
                "skill",
                0o644,
            ),
            (
                "skills/prove/SKILL.md",
                b"---\nname: prove\ndescription: Proof workflow\n---\n".to_vec(),
                "skill",
                0o644,
            ),
        ];
        for (path, bytes, _, mode) in &files {
            fs::write(source.join(path), bytes).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(source.join(path), fs::Permissions::from_mode(*mode)).unwrap();
            }
        }
        let inventory = inventory(&files);
        Self {
            root,
            project,
            source,
            inventory,
        }
    }

    pub fn confined(&self) -> ConfinedRoot {
        ConfinedRoot::open(&self.root).unwrap()
    }

    pub fn plan(&self) -> PackagePlan {
        plan_package_from_inventory(&self.source, &self.inventory).unwrap()
    }

    pub fn build(&self, relative: &str) -> PackageSnapshot {
        let plan = self.plan();
        let mut file = ScopedFile::new(self.confined(), relative).unwrap();
        build_package(&plan, &mut file).unwrap()
    }

    pub fn tree(&self) -> Vec<(String, Vec<u8>)> {
        tree(&self.root)
    }
}

fn unique_time() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

impl Drop for JourneyFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Serialize)]
struct Inventory<'a> {
    schema: &'static str,
    context_id: &'static str,
    candidate_id: &'static str,
    catalog_id: &'static str,
    plugin_id: &'static str,
    version: &'static str,
    source_date_epoch: u64,
    entries: Vec<Entry<'a>>,
}

#[derive(Serialize)]
struct Entry<'a> {
    path: &'a str,
    object_type: &'static str,
    mode: u32,
    sha256: String,
    byte_length: u64,
    role: &'a str,
}

pub fn runtime_probe_bytes() -> Vec<u8> {
    format!(
        "#!/bin/sh\ntest \"$1\" = --json && test \"$2\" = --help || exit 64\nsleep 0.15\n/bin/cat <<'HUL_HELP'\n{}\nHUL_HELP\n",
        crate::distribution::canonical_runtime_help_json(),
    )
    .into_bytes()
}

fn inventory(files: &[(&str, Vec<u8>, &str, u32)]) -> Vec<u8> {
    let entries = files
        .iter()
        .map(|(path, bytes, role, mode)| Entry {
            path,
            object_type: "regular-file",
            mode: *mode,
            sha256: digest(bytes),
            byte_length: bytes.len() as u64,
            role,
        })
        .collect();
    serde_json::to_vec(&Inventory {
        schema: "harness-ultragoal.accepted-package-source-set.v1",
        context_id: CONTEXT_ID,
        candidate_id: CANDIDATE_ID,
        catalog_id: CATALOG_ID,
        plugin_id: PLUGIN_ID,
        version: VERSION,
        source_date_epoch: 1_700_000_000,
        entries,
    })
    .unwrap()
}

pub fn write_scoped(root: ConfinedRoot, path: &str, bytes: &[u8]) -> ScopedFile {
    let file = ScopedFile::new(root, path).unwrap();
    assert!(file.apply(None, Some(bytes)).unwrap());
    file
}

pub fn renamed(path: &Path, suffix: &str) -> PathBuf {
    path.with_file_name(format!(
        "{}-{suffix}",
        path.file_name().unwrap().to_string_lossy()
    ))
}
