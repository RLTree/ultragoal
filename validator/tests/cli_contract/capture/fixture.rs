use crate::context::{BuildRequest, LiveContext};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

pub struct RepoFixture {
    root: PathBuf,
}

impl RepoFixture {
    pub fn new(label: &str) -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-capture-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create fixture root");
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("run git init");
        assert!(status.success(), "git init failed");
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write_script(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create script parent");
        }
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write script");
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(path, permissions).expect("make executable");
        }
    }

    pub fn write_file(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create file parent");
        }
        fs::write(path, bytes).expect("write file");
    }

    pub fn context(&self) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_repository_root(&self.root)
                .expect_worktree_root(&self.root)
                .probe_tool("sandbox-exec")
                .probe_tool("sh"),
        )
        .expect("build fixture context")
    }

    pub fn context_with_secret(&self, name: &str, public_version: &str) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(&self.root)
                .expect_repository_root(&self.root)
                .expect_worktree_root(&self.root)
                .bind_secret_source(name, public_version)
                .probe_tool("sandbox-exec")
                .probe_tool("sh"),
        )
        .expect("build secret-bound fixture context")
    }
}

impl Drop for RepoFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn read_command(program: &str) -> super::capture::CommandSpec {
    super::capture::CommandSpec::new(program, crate::context::EffectClass::Read)
}
