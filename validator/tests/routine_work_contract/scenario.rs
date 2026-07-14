use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::context::{BuildRequest, LiveContext};
use super::routine_fixture_workspace::claim_routine_fixture_root;
use super::routine_work::{
    CheckClass, CheckNode, ClaimBoundary, ImpactGraph, PathMatcher, PathRoute, RepoPath, RunnerSpec,
};

pub fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    pub fn new(label: &str) -> Self {
        let root = claim_routine_fixture_root(label);
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        let repo = Self { root };
        repo.git(&["init", "-q"]);
        repo.git(&["config", "user.email", "routine@example.invalid"]);
        repo.git(&["config", "user.name", "Routine Contract"]);
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 1 }\n");
        repo.write("tests/check.rs", b"#[test] fn check() {}\n");
        repo.write("docs/guide.md", b"guide\n");
        repo.write("release.json", b"{}\n");
        repo.git(&["add", "."]);
        repo.git(&["commit", "-q", "-m", "fixture"]);
        repo
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    pub fn remove(&self, relative: &str) {
        fs::remove_file(self.root.join(relative)).unwrap();
    }

    pub fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
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
            BuildRequest::new(&self.root)
                .bind_non_secret_configuration("profile", profile)
                .probe_tool("cargo")
                .probe_tool("definitely-missing-routine-accelerator"),
        )
        .unwrap()
    }

    pub fn tree(&self) -> BTreeMap<String, String> {
        let mut rows = BTreeMap::new();
        visit_tree(&self.root, &self.root, &mut rows);
        rows
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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

pub fn graph() -> ImpactGraph {
    graph_with_order(false)
}

pub fn graph_with_order(reverse: bool) -> ImpactGraph {
    let mut nodes = vec![
        node("syntax", &[], CheckClass::Routine, "git", None),
        node("compile", &["syntax"], CheckClass::Routine, "git", None),
        node("unit", &["compile"], CheckClass::Routine, "git", None),
        node(
            "release",
            &["unit"],
            CheckClass::ClaimBoundaryOnly,
            "git",
            None,
        ),
    ];
    let mut routes = vec![
        route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        ),
        route(
            "route-tests",
            PathMatcher::Prefix(path("tests")),
            &["unit"],
            false,
        ),
        route(
            "route-docs",
            PathMatcher::Prefix(path("docs")),
            &["syntax"],
            false,
        ),
        route(
            "route-release",
            PathMatcher::Exact(path("release.json")),
            &["release"],
            true,
        ),
    ];
    let mut claims = vec![
        ClaimBoundary::new("routine", vec!["unit".to_owned()]).unwrap(),
        ClaimBoundary::new("release", vec!["release".to_owned()]).unwrap(),
    ];
    if reverse {
        nodes.reverse();
        routes.reverse();
        claims.reverse();
    }
    ImpactGraph::new(nodes, routes, claims).unwrap()
}

pub fn fallback_graph() -> ImpactGraph {
    ImpactGraph::new(
        vec![node(
            "compile",
            &[],
            CheckClass::Routine,
            "definitely-missing-routine-accelerator",
            Some("git"),
        )],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap()
}

pub fn node(
    id: &str,
    dependencies: &[&str],
    class: CheckClass,
    primary: &str,
    fallback: Option<&str>,
) -> CheckNode {
    CheckNode::new(
        id,
        dependencies.iter().map(|value| (*value).to_owned()),
        class,
        RunnerSpec::new(primary, fallback.map(ToOwned::to_owned)).unwrap(),
    )
    .unwrap()
}

pub fn route(id: &str, matcher: PathMatcher, nodes: &[&str], strict: bool) -> PathRoute {
    PathRoute::new(
        id,
        matcher,
        nodes.iter().map(|value| (*value).to_owned()),
        strict,
    )
    .unwrap()
}

pub fn path(value: &str) -> RepoPath {
    RepoPath::parse(value).unwrap()
}
