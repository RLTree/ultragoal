use super::*;
use crate::catalog_fixture_scope::ClaimedFixtureScope;
use std::sync::{Mutex, MutexGuard};

pub(crate) const GRAPH_ID: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
pub(crate) const CANDIDATE_ID: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
pub(crate) const OTHER_CANDIDATE_ID: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";
pub(crate) const PLAN_ID: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";
pub(crate) const TRUE_TOOL_ID: &str =
    "sha256:5555555555555555555555555555555555555555555555555555555555555555";
pub(crate) const VALID_CATALOG: &[u8] =
    include_bytes!("../../../fixtures/routine-production-catalog/valid-catalog-v2.json");

pub(crate) static NEXT: AtomicU64 = AtomicU64::new(0);
static FIXTURE_ROOT_LOCK: Mutex<()> = Mutex::new(());

pub(crate) type FixtureRootGuard = MutexGuard<'static, ()>;

pub(crate) fn lock_fixture_root() -> FixtureRootGuard {
    FIXTURE_ROOT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

pub(crate) struct TestRoot {
    pub(crate) path: PathBuf,
    pub(super) scope: ClaimedFixtureScope,
}

impl TestRoot {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn write_catalog(&self, bytes: &[u8]) {
        fs::write(self.path.join("config/routines.json"), bytes).unwrap();
    }

    pub(crate) fn teardown_after_assertions(&mut self) {
        self.scope
            .teardown_after_assertions()
            .expect("catalog fixture descriptor-held teardown failed");
        assert!(
            !self.path.exists(),
            "catalog fixture teardown retained scope: {}",
            self.path.display()
        );
    }
}

pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn file_sha(path: &Path) -> String {
    sha(&fs::read(path).unwrap())
}

pub(crate) fn full_adoption(bytes: &[u8], candidate: &str) -> CatalogAdoption {
    CatalogAdoption::new(
        sha(bytes),
        bytes.len() as u64,
        GRAPH_ID,
        candidate,
        vec![
            AdoptedRoutineNode::new("syntax", Vec::<String>::new()).unwrap(),
            AdoptedRoutineNode::new("verify", vec!["syntax".to_owned()]).unwrap(),
        ],
    )
    .unwrap()
}

pub(crate) fn one_node_adoption(bytes: &[u8]) -> CatalogAdoption {
    CatalogAdoption::new(
        sha(bytes),
        bytes.len() as u64,
        GRAPH_ID,
        CANDIDATE_ID,
        vec![AdoptedRoutineNode::new("syntax", Vec::<String>::new()).unwrap()],
    )
    .unwrap()
}

pub(crate) fn load_full(root: &TestRoot, candidate: &str) -> ProductionRoutineCatalog {
    load_production_catalog(
        root.path(),
        Path::new("config/routines.json"),
        full_adoption(VALID_CATALOG, candidate),
    )
    .unwrap()
}

pub(crate) fn input(root: &TestRoot, relative: &str) -> TransitiveInputExpectation {
    let path = root.path().join(relative);
    TransitiveInputExpectation::new(relative, file_sha(&path), fs::metadata(path).unwrap().len())
        .unwrap()
}

#[cfg(unix)]
pub(crate) fn runner(tool: &str, tool_id: &str, path: &Path) -> RunnerObservation {
    let metadata = fs::metadata(path).unwrap();
    RunnerObservation::new(
        tool,
        tool_id,
        path,
        file_sha(path),
        metadata.len(),
        metadata.mode(),
    )
    .unwrap()
}

pub(crate) fn selected(root: &TestRoot, fallback: bool) -> Vec<SelectedRoutineNode> {
    assert!(!fallback, "catalog v2 has no fallback selection");
    vec![
        SelectedRoutineNode::new(
            "syntax",
            Vec::<String>::new(),
            sha(b"syntax input identity"),
            vec![input(root, "src/input.txt")],
        )
        .unwrap(),
        SelectedRoutineNode::new(
            "verify",
            vec!["syntax".to_owned()],
            sha(b"verify input identity"),
            vec![input(root, "src/input.txt"), input(root, "tests/input.txt")],
        )
        .unwrap(),
    ]
}

#[cfg(unix)]
pub(crate) fn request(
    catalog: &ProductionRoutineCatalog,
    root: &TestRoot,
    candidate: &str,
    fallback: bool,
) -> CatalogSelectionRequest {
    assert!(!fallback, "catalog v2 has no fallback selection");
    let runners = vec![runner(
        "ultragoal",
        TRUE_TOOL_ID,
        Path::new("/usr/bin/true"),
    )];
    CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        candidate,
        PLAN_ID,
        selected(root, fallback),
        runners,
    )
    .unwrap()
}

pub(crate) fn load_raw(root: &TestRoot, bytes: &[u8], adoption: CatalogAdoption) -> &'static str {
    root.write_catalog(bytes);
    load_production_catalog(root.path(), Path::new("config/routines.json"), adoption)
        .unwrap_err()
        .code()
}

pub(crate) fn tree(root: &Path) -> BTreeMap<String, String> {
    fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, String>) {
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
            if metadata.file_type().is_dir() {
                rows.insert(relative, "directory".to_owned());
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.insert(
                    relative,
                    format!(
                        "symlink:{}",
                        sha(fs::read_link(&path).unwrap().to_string_lossy().as_bytes())
                    ),
                );
            } else if metadata.file_type().is_file() {
                rows.insert(relative, format!("file:{}", file_sha(&path)));
            } else {
                rows.insert(relative, "special".to_owned());
            }
        }
    }
    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}

pub(crate) fn git(root: &Path, arguments: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}");
    output.stdout
}
