use super::{PackageCapture, PackageEntryKind, PackageSnapshot};
use crate::context::{BuildRequest, LiveContext};
use std::fs;
use std::path::{Path, PathBuf};

struct Repo {
    pub(super) root: PathBuf,
}

impl Repo {
    pub(super) fn new(label: &str) -> Self {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
        fs::create_dir_all(root.join(".codex-plugin")).expect("plugin manifest dir");
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"snapshot-test","version":"0.0.0","skills":"./skills/"}"#,
        )
        .expect("plugin manifest");
        fs::create_dir_all(root.join("schemas")).expect("schema dir");
        fs::write(root.join("schemas/catalog.json"), "{}\n").expect("catalog");
        fs::write(root.join("resource.txt"), "trusted\n").expect("resource");
        write_manifest(&root, &["plugin-manifest-draft.json", "resource.txt"]);
        repository_fixture_adapter::initialize(&root).expect("git init");
        Self { root }
    }

    pub(super) fn context(&self) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_worktree_root(&self.root)
                .expect_repository_root(&self.root),
        )
        .expect("live context")
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_manifest(root: &Path, resources: &[&str]) {
    manifest_fixture_codec::write(manifest_fixture_codec::ManifestFixtureRequest {
        root,
        resources,
    })
    .expect("manifest");
}

fn status(root: &Path) -> Vec<u8> {
    repository_fixture_adapter::status(root).expect("git status")
}

#[path = "manifest/behavior.rs"]
mod manifest_behavior;
#[path = "manifest/fixture_codec.rs"]
mod manifest_fixture_codec;
mod mode_revalidation;
mod repository_fixture_adapter;
#[path = "special/entries.rs"]
mod special_entries;
#[path = "special/file_adapter.rs"]
mod special_file_adapter;
