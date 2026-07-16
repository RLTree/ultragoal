use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::Path;
use std::process::Command;

use super::context::{BuildRequest, LiveContext};
use super::routine_fixture_workspace::{
    RoutineFixtureClaim, RoutineFixtureOwner, RoutineFixtureTeardown, claim_routine_fixture_root,
};

pub fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub struct TempRepo {
    claim: RoutineFixtureClaim,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineFixtureInitializationFailure {
    id: &'static str,
}

impl TempRepo {
    pub fn new(label: &str) -> Self {
        let owner = RoutineFixtureOwner::new();
        Self::new_in_invocation(label, &owner)
    }

    pub(crate) fn new_in_invocation(label: &str, owner: &RoutineFixtureOwner) -> Self {
        let claim = claim_routine_fixture_root(label, owner);
        let mut repo = Self { claim };
        let result = catch_unwind(AssertUnwindSafe(|| repo.initialize_repository()));
        if let Err(payload) = result {
            repo.teardown_after_assertions();
            resume_unwind(payload);
        }
        repo
    }

    pub(crate) fn interrupted_initialization(
        label: &str,
    ) -> Result<Self, RoutineFixtureInitializationFailure> {
        let owner = RoutineFixtureOwner::new();
        let claim = claim_routine_fixture_root(label, &owner);
        let mut repo = Self { claim };
        repo.write("partial-setup.txt", b"partial setup\n");
        repo.teardown_after_assertions();
        Err(RoutineFixtureInitializationFailure {
            id: "routine-fixture-initialization-interrupted",
        })
    }

    fn initialize_repository(&self) {
        fs::create_dir(self.root().join("src")).unwrap();
        fs::create_dir(self.root().join("tests")).unwrap();
        fs::create_dir(self.root().join("docs")).unwrap();
        self.git(&["init", "-q"]);
        self.git(&["config", "user.email", "routine@example.invalid"]);
        self.git(&["config", "user.name", "Routine Contract"]);
        self.write("src/lib.rs", b"pub fn value() -> u8 { 1 }\n");
        self.write("tests/check.rs", b"#[test] fn check() {}\n");
        self.write("docs/guide.md", b"guide\n");
        self.write("release.json", b"{}\n");
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", "fixture"]);
    }

    pub fn root(&self) -> &Path {
        self.claim.path()
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    pub fn remove(&self, relative: &str) {
        fs::remove_file(self.root().join(relative)).unwrap();
    }

    pub fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.root())
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
        output.stdout
    }

    pub fn status(&self) -> Vec<u8> {
        self.git(&[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
        ])
    }

    pub fn context(&self, profile: &str) -> LiveContext {
        LiveContext::build(
            BuildRequest::new(self.root())
                .bind_non_secret_configuration("profile", profile)
                .probe_tool("cargo")
                .probe_tool("definitely-missing-routine-accelerator"),
        )
        .unwrap()
    }

    pub fn tree(&self) -> BTreeMap<String, String> {
        let mut rows = BTreeMap::new();
        visit_tree(self.root(), self.root(), &mut rows);
        rows
    }

    // This is an explicit, quiescent test-fixture action after every actor and
    // assertion. It is never called from Drop and makes no concurrent mutation claim.
    pub fn teardown_after_assertions(&mut self) {
        self.try_teardown_after_assertions().assert_removed();
    }

    pub fn try_teardown_after_assertions(&mut self) -> RoutineFixtureTeardown {
        self.claim.teardown_quiescent()
    }
}

impl RoutineFixtureInitializationFailure {
    pub(crate) fn id(&self) -> &'static str {
        self.id
    }
}

fn visit_tree(root: &Path, current: &Path, rows: &mut BTreeMap<String, String>) {
    let mut entries = fs::read_dir(current)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.is_dir() {
            rows.insert(relative, "directory".to_owned());
            visit_tree(root, &path, rows);
        } else if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).unwrap();
            rows.insert(
                relative,
                format!("symlink:{}", sha(target.to_string_lossy().as_bytes())),
            );
        } else if metadata.is_file() {
            rows.insert(relative, format!("file:{}", sha(&fs::read(&path).unwrap())));
        } else {
            rows.insert(relative, "special".to_owned());
        }
    }
}

#[cfg(test)]
mod initialization_tests {
    use super::TempRepo;

    #[test]
    fn complete_initialization_is_explicitly_torn_down() {
        let mut repo = TempRepo::new("shared-complete-initialization");
        assert!(repo.status().is_empty());
        repo.write("remove-me.txt", b"owned transient\n");
        repo.remove("remove-me.txt");
        repo.teardown_after_assertions();
    }

    #[test]
    fn interrupted_initialization_returns_typed_failure_after_rollback() {
        let failure = match TempRepo::interrupted_initialization("shared-initialization-failure") {
            Err(failure) => failure,
            Ok(_) => panic!("interrupted initialization must fail"),
        };
        assert_eq!(failure.id(), "routine-fixture-initialization-interrupted");
    }
}
