#[path = "../src/routine_work/catalog.rs"]
mod catalog;

use catalog::{
    AdoptedRoutineNode, CatalogAdoption, CatalogSelectionRequest, ProductionRoutineCatalog,
    RunnerObservation, SelectedRoutineNode, TransitiveInputExpectation, load_production_catalog,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::orchestration::{
    Actor, ArtifactWorkspace, Binding, CanonicalPath, EffectClass, EffectGrant, LeaseSpec,
    OwnedScope, PrerequisiteEvidence, Principal, SafetyClass, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const GRAPH_ID: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const CANDIDATE_ID: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const OTHER_CANDIDATE_ID: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";
const PLAN_ID: &str = "sha256:4444444444444444444444444444444444444444444444444444444444444444";
const TRUE_TOOL_ID: &str =
    "sha256:5555555555555555555555555555555555555555555555555555555555555555";
const FALSE_TOOL_ID: &str =
    "sha256:6666666666666666666666666666666666666666666666666666666666666666";
const TRUE_PROGRAM_SHA256: &str =
    "sha256:b9b54a7e5d45dda1aca284b454829f7f0bc76a827565f32418db2fb7869970eb";
const FALSE_PROGRAM_SHA256: &str =
    "sha256:0c5fb690df52f914a97ef76bc50baebba4e506511844d618e1ddf0611db1df22";
const SYSTEM_PROGRAM_BYTE_LENGTH: u64 = 84_032;
const SYSTEM_PROGRAM_UNIX_MODE: u32 = 0o100755;
const R3_CONTEXT_ID: &str =
    "sha256:063fba8de7f4c543180c5842169a9c0915c25aa6d98199eeeae37ef4a62b0132";
const R3_CANDIDATE_ID: &str =
    "sha256:f4228656572628dd481187f62513dfd277526f8d910e782b3d673460521043e6";
const R3_RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/ROUTINE-PRODUCTION-CATALOG-073.json";
const R3_WORK_PACKAGE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/ROUTINE-PRODUCTION-CATALOG-073-R3.json";
const R3_WORK_PACKAGE_SHA256: &str =
    "sha256:5c60b9b3c4622eb5f7f0d2d3e5a8c0610896cb9a31a0db97f1d6b6a6e04d1ab1";
const VALID_CATALOG: &[u8] =
    include_bytes!("../../fixtures/routine-production-catalog/valid-catalog.json");

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new(label: &str, catalog_bytes: &[u8]) -> Self {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(format!(
            "/private/tmp/hul-routine-production-catalog-073-r3-scratch/test-fixtures/{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("config")).unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::create_dir_all(path.join("tests")).unwrap();
        fs::create_dir_all(path.join("target/routine-syntax")).unwrap();
        fs::create_dir_all(path.join("target/routine-verify")).unwrap();
        fs::write(path.join("config/routines.json"), catalog_bytes).unwrap();
        fs::write(path.join("src/input.txt"), b"source input\n").unwrap();
        fs::write(path.join("tests/input.txt"), b"test input\n").unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_catalog(&self, bytes: &[u8]) {
        fs::write(self.path.join("config/routines.json"), bytes).unwrap();
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn file_sha(path: &Path) -> String {
    sha(&fs::read(path).unwrap())
}

fn full_adoption(bytes: &[u8], candidate: &str) -> CatalogAdoption {
    CatalogAdoption::new(
        sha(bytes),
        bytes.len() as u64,
        GRAPH_ID,
        candidate,
        vec![
            AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap(),
            AdoptedRoutineNode::new(
                "verify",
                vec!["syntax".to_owned()],
                "true",
                Some("false".to_owned()),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn one_node_adoption(bytes: &[u8]) -> CatalogAdoption {
    CatalogAdoption::new(
        sha(bytes),
        bytes.len() as u64,
        GRAPH_ID,
        CANDIDATE_ID,
        vec![AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap()],
    )
    .unwrap()
}

fn load_full(root: &TestRoot, candidate: &str) -> ProductionRoutineCatalog {
    load_production_catalog(
        root.path(),
        Path::new("config/routines.json"),
        full_adoption(VALID_CATALOG, candidate),
    )
    .unwrap()
}

fn input(root: &TestRoot, relative: &str) -> TransitiveInputExpectation {
    let path = root.path().join(relative);
    TransitiveInputExpectation::new(relative, file_sha(&path), fs::metadata(path).unwrap().len())
        .unwrap()
}

#[cfg(unix)]
fn runner(tool: &str, tool_id: &str, path: &Path) -> RunnerObservation {
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

fn selected(root: &TestRoot, fallback: bool) -> Vec<SelectedRoutineNode> {
    vec![
        SelectedRoutineNode::new(
            "syntax",
            Vec::<String>::new(),
            "true",
            TRUE_TOOL_ID,
            false,
            sha(b"syntax input identity"),
            vec![input(root, "src/input.txt")],
        )
        .unwrap(),
        SelectedRoutineNode::new(
            "verify",
            vec!["syntax".to_owned()],
            if fallback { "false" } else { "true" },
            if fallback {
                FALSE_TOOL_ID
            } else {
                TRUE_TOOL_ID
            },
            fallback,
            sha(b"verify input identity"),
            vec![input(root, "src/input.txt"), input(root, "tests/input.txt")],
        )
        .unwrap(),
    ]
}

#[cfg(unix)]
fn request(
    catalog: &ProductionRoutineCatalog,
    root: &TestRoot,
    candidate: &str,
    fallback: bool,
) -> CatalogSelectionRequest {
    let mut runners = vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))];
    if fallback {
        runners.push(runner("false", FALSE_TOOL_ID, Path::new("/usr/bin/false")));
    }
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

fn load_raw(root: &TestRoot, bytes: &[u8], adoption: CatalogAdoption) -> &'static str {
    root.write_catalog(bytes);
    load_production_catalog(root.path(), Path::new("config/routines.json"), adoption)
        .unwrap_err()
        .code()
}

fn tree(root: &Path) -> BTreeMap<String, String> {
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

fn git(root: &Path, arguments: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}");
    output.stdout
}

fn commit_fixture(root: &Path) {
    git(root, &["init", "-q"]);
    git(
        root,
        &["config", "user.email", "routine-catalog@example.invalid"],
    );
    git(root, &["config", "user.name", "Routine Catalog Contract"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "fixture"]);
}

fn status(root: &Path) -> Vec<u8> {
    git(
        root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
        ],
    )
}

fn one_node_catalog(
    arguments: &[String],
    environment: &[(String, String)],
    read_sources: &[String],
    output_scopes: &[String],
    working_directory: &str,
) -> Vec<u8> {
    let environment = environment.iter().cloned().collect::<BTreeMap<_, _>>();
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "RoutineProductionCatalog-v1",
        "graph_id": GRAPH_ID,
        "routines": [{
            "definition_id": "routine.syntax.v1",
            "node_id": "syntax",
            "depends_on": [],
            "working_directory": working_directory,
            "runner_policy": "immutable-single-process-exact-executable-v1",
            "read_policy": "selected-transitive-exact-regular-files-v1",
            "read_sources": read_sources,
            "environment": environment,
            "timeout_ms": 60000,
            "output_budget_bytes": 4194304,
            "output_scopes": output_scopes,
            "primary": {
                "tool": "true",
                "tool_identity_sha256": TRUE_TOOL_ID,
                "executable_path": "/usr/bin/true",
                "program_sha256": TRUE_PROGRAM_SHA256,
                "program_byte_length": SYSTEM_PROGRAM_BYTE_LENGTH,
                "program_unix_mode": SYSTEM_PROGRAM_UNIX_MODE,
                "arguments": arguments
            }
        }]
    }))
    .unwrap()
}

#[test]
fn fixture_catalog_declares_the_exact_adversarial_matrix_without_claim_effect() {
    let cases: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../fixtures/routine-production-catalog/cases.json"
    ))
    .unwrap();
    assert_eq!(cases["schema_version"], "RoutineProductionCatalogCases-v1");
    assert_eq!(cases["claim_effect"], "none");
    assert_eq!(cases["cases"].as_array().unwrap().len(), 16);
    let ids = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 16);
}

#[cfg(unix)]
#[test]
fn valid_catalog_binds_exact_primary_invocations_deterministically_and_without_writes() {
    let root = TestRoot::new("valid-primary", VALID_CATALOG);
    let before = tree(root.path());
    let first_catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(first_catalog.definition_count(), 2);
    assert_eq!(
        first_catalog.definition_ids().collect::<Vec<_>>(),
        ["routine.syntax.v1", "routine.verify.v1"]
    );
    let first = first_catalog
        .bind_selected(request(&first_catalog, &root, CANDIDATE_ID, false))
        .unwrap();
    let second_catalog = load_full(&root, CANDIDATE_ID);
    let second = second_catalog
        .bind_selected(request(&second_catalog, &root, CANDIDATE_ID, false))
        .unwrap();
    assert_eq!(first.invocation_set_id(), second.invocation_set_id());
    assert_eq!(first, second);
    assert_eq!(
        first
            .invocations()
            .iter()
            .map(|row| row.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "verify"]
    );
    assert!(first.invocations().iter().all(|row| {
        row.selected_tool() == "true"
            && row.environment()["LANG"] == "C"
            && row.environment()["LC_ALL"] == "C"
            && row.environment()["PATH"] == "/usr/bin"
            && row.arguments() == ["--version"]
            && !row.read_sources().is_empty()
            && row.timeout_ms() > 0
            && row.output_budget_bytes() > 0
            && row
                .output_scopes()
                .iter()
                .all(|scope| scope.relative_path().starts_with("target/"))
    }));
    assert_eq!(tree(root.path()), before);
}

#[cfg(unix)]
#[test]
fn conservative_fallback_requires_the_adopted_equivalent_recipe_and_exact_runner() {
    let root = TestRoot::new("valid-fallback", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let bound = catalog
        .bind_selected(request(&catalog, &root, CANDIDATE_ID, true))
        .unwrap();
    assert_eq!(bound.invocations()[0].selected_tool(), "true");
    assert_eq!(bound.invocations()[1].selected_tool(), "false");
}

#[cfg(unix)]
#[test]
fn exact_same_spelling_same_authority_reuse_selects_and_binds_both_definitions() {
    let root = TestRoot::new("exact-runner-reuse", VALID_CATALOG);
    let before = tree(root.path());
    let catalog = load_full(&root, CANDIDATE_ID);
    let bound = catalog
        .bind_selected(request(&catalog, &root, CANDIDATE_ID, false))
        .unwrap();

    assert_eq!(bound.invocations().len(), 2);
    assert!(
        bound
            .invocations()
            .iter()
            .all(|invocation| invocation.selected_tool() == "true")
    );
    assert_eq!(tree(root.path()), before);
}

#[test]
fn unknown_duplicate_ambiguous_and_missing_definitions_fail_closed() {
    let case_aliased_fallback = AdoptedRoutineNode::new(
        "verify",
        vec!["syntax".to_owned()],
        "true",
        Some("TRUE".to_owned()),
    )
    .unwrap_err();
    assert_eq!(
        case_aliased_fallback.code(),
        "catalog-adoption-runner-duplicated"
    );

    let unknown = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"unknown\"", 1)
        .into_bytes();
    let root = TestRoot::new("unknown-definition", &unknown);
    assert_eq!(
        load_raw(&root, &unknown, full_adoption(&unknown, CANDIDATE_ID)),
        "catalog-definition-node-unknown"
    );

    let duplicate = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"syntax\"", 1)
        .into_bytes();
    let root = TestRoot::new("duplicate-definition", &duplicate);
    assert!(matches!(
        load_raw(&root, &duplicate, full_adoption(&duplicate, CANDIDATE_ID)),
        "catalog-definition-node-ambiguous" | "catalog-definition-node-duplicated"
    ));

    let ambiguous = String::from_utf8(VALID_CATALOG.to_vec())
        .unwrap()
        .replacen("\"node_id\": \"verify\"", "\"node_id\": \"Syntax\"", 1)
        .into_bytes();
    let root = TestRoot::new("ambiguous-definition", &ambiguous);
    assert_eq!(
        load_raw(&root, &ambiguous, full_adoption(&ambiguous, CANDIDATE_ID)),
        "catalog-definition-node-ambiguous"
    );

    let mut missing: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    missing["routines"].as_array_mut().unwrap().pop();
    let missing = serde_json::to_vec_pretty(&missing).unwrap();
    let root = TestRoot::new("missing-definition", &missing);
    assert_eq!(
        load_raw(&root, &missing, full_adoption(&missing, CANDIDATE_ID)),
        "catalog-definition-set-inexact"
    );

    let mut conflicting_runner_authority: serde_json::Value =
        serde_json::from_slice(VALID_CATALOG).unwrap();
    conflicting_runner_authority["routines"][1]["primary"]["program_sha256"] =
        serde_json::json!(FALSE_PROGRAM_SHA256);
    let conflicting_runner_authority =
        serde_json::to_vec_pretty(&conflicting_runner_authority).unwrap();
    let root = TestRoot::new(
        "conflicting-runner-authority",
        &conflicting_runner_authority,
    );
    assert_eq!(
        load_raw(
            &root,
            &conflicting_runner_authority,
            full_adoption(&conflicting_runner_authority, CANDIDATE_ID),
        ),
        "catalog-runner-authority-ambiguous"
    );

    let mut case_alias_same_authority: serde_json::Value =
        serde_json::from_slice(VALID_CATALOG).unwrap();
    case_alias_same_authority["routines"][1]["primary"]["tool"] = serde_json::json!("TRUE");
    let case_alias_same_authority = serde_json::to_vec_pretty(&case_alias_same_authority).unwrap();
    let case_alias_adoption = |bytes: &[u8]| {
        CatalogAdoption::new(
            sha(bytes),
            bytes.len() as u64,
            GRAPH_ID,
            CANDIDATE_ID,
            vec![
                AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap(),
                AdoptedRoutineNode::new(
                    "verify",
                    vec!["syntax".to_owned()],
                    "TRUE",
                    Some("false".to_owned()),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    };
    let root = TestRoot::new("case-alias-same-authority", &case_alias_same_authority);
    assert_eq!(
        load_raw(
            &root,
            &case_alias_same_authority,
            case_alias_adoption(&case_alias_same_authority),
        ),
        "catalog-runner-spelling-ambiguous"
    );

    let mut case_alias_conflicting_authority: serde_json::Value =
        serde_json::from_slice(&case_alias_same_authority).unwrap();
    case_alias_conflicting_authority["routines"][1]["primary"]["program_sha256"] =
        serde_json::json!(FALSE_PROGRAM_SHA256);
    let case_alias_conflicting_authority =
        serde_json::to_vec_pretty(&case_alias_conflicting_authority).unwrap();
    let root = TestRoot::new(
        "case-alias-conflicting-authority",
        &case_alias_conflicting_authority,
    );
    assert_eq!(
        load_raw(
            &root,
            &case_alias_conflicting_authority,
            case_alias_adoption(&case_alias_conflicting_authority),
        ),
        "catalog-runner-spelling-ambiguous"
    );
}

#[cfg(unix)]
#[test]
fn definition_candidate_input_and_dependency_drift_after_parse_are_refused() {
    let root = TestRoot::new("definition-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    root.write_catalog(
        &String::from_utf8(VALID_CATALOG.to_vec())
            .unwrap()
            .replace("\"timeout_ms\": 60000", "\"timeout_ms\": 60001")
            .into_bytes(),
    );
    assert_eq!(
        catalog.verify_current().unwrap_err().code(),
        "catalog-sealed-file-changed"
    );

    let root = TestRoot::new("runner-expectation-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    root.write_catalog(
        &String::from_utf8(VALID_CATALOG.to_vec())
            .unwrap()
            .replacen(TRUE_PROGRAM_SHA256, FALSE_PROGRAM_SHA256, 1)
            .into_bytes(),
    );
    assert_eq!(
        catalog.verify_current().unwrap_err().code(),
        "catalog-sealed-file-changed"
    );

    let root = TestRoot::new("candidate-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let error = catalog
        .bind_selected(request(&catalog, &root, OTHER_CANDIDATE_ID, false))
        .unwrap_err();
    assert_eq!(error.code(), "catalog-selection-binding-stale");

    let root = TestRoot::new("input-drift", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale_selected = selected(&root, false);
    fs::write(root.path().join("tests/input.txt"), b"mutated after plan\n").unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale_selected,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-transitive-input-stale"
    );

    let root = TestRoot::new("dependency-omission", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let verify_only = selected(&root, false).pop().unwrap();
    assert_eq!(
        CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            vec![verify_only],
            vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
        )
        .unwrap_err()
        .code(),
        "catalog-selection-dependency-closure-incomplete"
    );
}

#[cfg(unix)]
#[test]
fn source_path_escape_symlink_hardlink_special_and_non_utf8_are_refused_without_blocking() {
    let root = TestRoot::new("source-path-security", VALID_CATALOG);
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("../outside.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-repository-relative-path-required"
    );

    let original = root.path().join("config/original.json");
    fs::rename(root.path().join("config/routines.json"), &original).unwrap();
    std::os::unix::fs::symlink("original.json", root.path().join("config/routines.json")).unwrap();
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    fs::remove_file(root.path().join("config/routines.json")).unwrap();
    fs::hard_link(&original, root.path().join("config/routines.json")).unwrap();
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    fs::remove_file(root.path().join("config/routines.json")).unwrap();
    let fifo = CString::new(root.path().join("config/routines.json").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert_eq!(
        load_production_catalog(
            root.path(),
            Path::new("config/routines.json"),
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-file-object-unsafe"
    );

    let non_utf8 = PathBuf::from(std::ffi::OsString::from_vec(vec![b'c', 0xff]));
    assert_eq!(
        load_production_catalog(
            root.path(),
            &non_utf8,
            full_adoption(VALID_CATALOG, CANDIDATE_ID),
        )
        .unwrap_err()
        .code(),
        "catalog-source-path-not-utf8"
    );
}

#[cfg(unix)]
#[test]
fn transitive_input_symlink_hardlink_and_special_file_are_refused() {
    let root = TestRoot::new("input-symlink", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::rename(
        root.path().join("tests/input.txt"),
        root.path().join("tests/original.txt"),
    )
    .unwrap();
    std::os::unix::fs::symlink("original.txt", root.path().join("tests/input.txt")).unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("input-hardlink", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::hard_link(
        root.path().join("tests/input.txt"),
        root.path().join("tests/input-alias.txt"),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("input-special", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let stale = selected(&root, false);
    fs::remove_file(root.path().join("tests/input.txt")).unwrap();
    let fifo = CString::new(root.path().join("tests/input.txt").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        stale,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-file-object-unsafe"
    );

    let root = TestRoot::new("output-symlink", VALID_CATALOG);
    fs::create_dir_all(root.path().join("target/alias-destination")).unwrap();
    fs::remove_dir(root.path().join("target/routine-verify")).unwrap();
    std::os::unix::fs::symlink(
        "alias-destination",
        root.path().join("target/routine-verify"),
    )
    .unwrap();
    let catalog = load_full(&root, CANDIDATE_ID);
    let request = self::request(&catalog, &root, CANDIDATE_ID, false);
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-output-scope-unsafe"
    );
}

#[test]
fn catalog_argv_environment_input_and_output_bounds_are_enforced() {
    let arguments = (0..129)
        .map(|index| format!("arg-{index}"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &arguments,
        &[],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("argv-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-arguments-invalid"
    );

    let environment = (0..62)
        .map(|index| (format!("ROUTINE_{index}"), "v".to_owned()))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &environment,
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("environment-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-environment-invalid"
    );

    let reads = (0..129)
        .map(|index| format!("src/input-{index}.txt"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &reads,
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("read-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-read-source-limit-exceeded"
    );

    let outputs = (0..129)
        .map(|index| format!("target/routine-{index}"))
        .collect::<Vec<_>>();
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["src/input.txt".to_owned()],
        &outputs,
        ".",
    );
    let root = TestRoot::new("output-bound", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-output-scope-limit-exceeded"
    );

    let too_large =
        TransitiveInputExpectation::new("src/input.txt", sha(b"x"), 64 * 1024 * 1024 + 1)
            .unwrap_err();
    assert_eq!(too_large.code(), "catalog-input-length-invalid");

    let oversized_catalog = vec![b'#'; 1024 * 1024 + 1];
    let root = TestRoot::new("catalog-bound", &oversized_catalog);
    assert_eq!(
        CatalogAdoption::new(
            sha(&oversized_catalog),
            oversized_catalog.len() as u64,
            GRAPH_ID,
            CANDIDATE_ID,
            vec![AdoptedRoutineNode::new("syntax", Vec::<String>::new(), "true", None).unwrap()],
        )
        .unwrap_err()
        .code(),
        "catalog-source-length-invalid"
    );
    drop(root);
}

#[test]
fn unsafe_workdir_loader_environment_path_overlap_and_prose_substitution_fail_closed() {
    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        "..",
    );
    let root = TestRoot::new("unsafe-workdir", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-working-directory-unsafe"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[("DYLD_INSERT_LIBRARIES".to_owned(), "/tmp/x".to_owned())],
        &["src/input.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("unsafe-environment", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-environment-invalid"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["target/input.txt".to_owned()],
        &["target".to_owned()],
        ".",
    );
    let root = TestRoot::new("overlapping-output", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-output-scope-overlaps-input"
    );

    let bytes = one_node_catalog(
        &["--version".to_owned()],
        &[],
        &["../secret.txt".to_owned()],
        &["target/routine-syntax".to_owned()],
        ".",
    );
    let root = TestRoot::new("read-path-escape", &bytes);
    assert_eq!(
        load_raw(&root, &bytes, one_node_adoption(&bytes)),
        "catalog-repository-relative-path-required"
    );

    let mut prose: serde_json::Value = serde_json::from_slice(VALID_CATALOG).unwrap();
    prose
        .as_object_mut()
        .unwrap()
        .insert("proof_receipt".to_owned(), serde_json::json!("PASS"));
    let prose = serde_json::to_vec_pretty(&prose).unwrap();
    let root = TestRoot::new("prose-substitution", &prose);
    assert_eq!(
        load_raw(&root, &prose, full_adoption(&prose, CANDIDATE_ID)),
        "catalog-source-invalid-json"
    );

    let digest_only = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "RoutineProductionCatalog-v1",
        "graph_id": GRAPH_ID,
        "digest": sha(b"definition prose"),
        "routines": []
    }))
    .unwrap();
    let root = TestRoot::new("digest-only-substitution", &digest_only);
    assert_eq!(
        load_raw(&root, &digest_only, one_node_adoption(&digest_only)),
        "catalog-source-invalid-json"
    );
}

#[cfg(unix)]
#[test]
fn symbolic_tool_executable_content_mode_and_identity_substitutions_are_refused() {
    let root = TestRoot::new("runner-substitution", VALID_CATALOG);
    let mutable_program = root.path().join("mutable-runner");
    fs::write(&mutable_program, fs::read("/usr/bin/true").unwrap()).unwrap();
    fs::set_permissions(&mutable_program, fs::Permissions::from_mode(0o700)).unwrap();
    let catalog = load_full(&root, CANDIDATE_ID);

    let caller_consistent_false = runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/false"));
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![caller_consistent_false],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );

    let metadata = fs::metadata("/usr/bin/true").unwrap();
    let forged = RunnerObservation::new(
        "true",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        sha(b"different executable"),
        metadata.len(),
        metadata.mode(),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );

    let forged_mode = RunnerObservation::new(
        "true",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        TRUE_PROGRAM_SHA256,
        SYSTEM_PROGRAM_BYTE_LENGTH,
        SYSTEM_PROGRAM_UNIX_MODE ^ 0o100,
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged_mode],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );

    let alternate_selected = vec![
        SelectedRoutineNode::new(
            "syntax",
            Vec::<String>::new(),
            "true",
            OTHER_CANDIDATE_ID,
            false,
            sha(b"syntax input identity"),
            vec![input(&root, "src/input.txt")],
        )
        .unwrap(),
        SelectedRoutineNode::new(
            "verify",
            vec!["syntax".to_owned()],
            "true",
            OTHER_CANDIDATE_ID,
            false,
            sha(b"verify input identity"),
            vec![
                input(&root, "src/input.txt"),
                input(&root, "tests/input.txt"),
            ],
        )
        .unwrap(),
    ];
    let alternate_runner = runner("true", OTHER_CANDIDATE_ID, Path::new("/usr/bin/true"));
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        alternate_selected,
        vec![alternate_runner],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-runner-authority-stale"
    );

    let mutable = runner("true", TRUE_TOOL_ID, &mutable_program);
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![mutable],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-runner-authority-stale"
    );
}

#[cfg(unix)]
#[test]
fn omitted_or_injected_transitive_inputs_and_definition_only_targets_cannot_pass() {
    let root = TestRoot::new("input-set-inexact", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let mut rows = selected(&root, false);
    rows[1] = SelectedRoutineNode::new(
        "verify",
        vec!["syntax".to_owned()],
        "true",
        TRUE_TOOL_ID,
        false,
        sha(b"verify input identity"),
        vec![input(&root, "src/input.txt")],
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        rows,
        vec![runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true"))],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-transitive-input-set-inexact"
    );

    assert_eq!(
        CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .code(),
        "catalog-selection-cardinality-invalid"
    );

    let extra_runner = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![
            runner("true", TRUE_TOOL_ID, Path::new("/usr/bin/true")),
            runner("false", FALSE_TOOL_ID, Path::new("/usr/bin/false")),
        ],
    )
    .unwrap_err();
    assert_eq!(
        extra_runner.code(),
        "catalog-runner-observation-set-inexact"
    );
}

#[cfg(unix)]
#[test]
fn parse_query_and_refusal_paths_are_recursively_zero_write() {
    let root = TestRoot::new("zero-write", VALID_CATALOG);
    commit_fixture(root.path());
    let before = tree(root.path());
    let before_status = status(root.path());
    let catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(catalog.graph_id(), GRAPH_ID);
    assert_eq!(catalog.definition_count(), 2);
    assert_eq!(tree(root.path()), before);
    assert_eq!(status(root.path()), before_status);

    let metadata = fs::metadata("/usr/bin/true").unwrap();
    let forged = RunnerObservation::new(
        "true",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        sha(b"forged"),
        metadata.len(),
        metadata.mode(),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged],
    )
    .unwrap();
    assert!(catalog.bind_selected(request).is_err());
    assert_eq!(tree(root.path()), before);
    assert_eq!(status(root.path()), before_status);
}

#[test]
fn corrective_worker_result_is_typed_bound_artifact_verified_and_substitution_safe() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert_eq!(
        file_sha(&repository_root.join(R3_WORK_PACKAGE_PATH)),
        R3_WORK_PACKAGE_SHA256,
        "receipt is bound to the exact root-issued R3 work package"
    );
    let result_bytes = fs::read(repository_root.join(R3_RESULT_PATH)).unwrap();
    let result =
        WorkerResultV1::parse_json(&result_bytes).expect("authoritative WorkerResultV1 parser");
    let (package, lease, policy) = corrective_lease();
    package.validate().expect("typed corrective work package");
    policy.validate().expect("typed corrective scope policy");
    lease
        .validate(&policy)
        .expect("typed corrective lease and scope binding");
    result
        .validate_for(&lease, &package)
        .expect("typed corrective WorkerResult binding");

    let before = result
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                fs::read(repository_root.join(&artifact.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let workspace = ArtifactWorkspace::new(repository_root).unwrap();
    let verified = workspace
        .verify(&result, &lease, &package)
        .expect("ArtifactWorkspace exact artifact verification");
    let result_id = result.result_id().expect("canonical result identity");
    assert_eq!(verified.result_id(), result_id);
    assert_eq!(verified.artifact_count(), 3);
    assert!(r3_artifact_set_is_exact(&result));
    assert_eq!(result.context_id, R3_CONTEXT_ID);
    assert_eq!(result.candidate_identity["context_id"], R3_CONTEXT_ID);
    assert_eq!(result.candidate_identity["candidate_id"], R3_CANDIDATE_ID);
    assert_eq!(
        result.candidate_identity["issued_root_commit"],
        "1750d586f6561d9d6ba64887cdafa6a36f23e41e"
    );
    assert_eq!(
        result.candidate_identity["issued_root_tree"],
        "bf2d3e53c4b9410df41fb94ddf9b56ea4cf1e58a"
    );
    assert_eq!(
        result.candidate_identity["work_package_sha256"],
        R3_WORK_PACKAGE_SHA256
    );
    let after = result
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                fs::read(repository_root.join(&artifact.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(after, before, "receipt verification must be zero-write");

    let mut unknown: serde_json::Value = serde_json::from_slice(&result_bytes).unwrap();
    unknown["proof_receipt"] = serde_json::json!("PASS");
    assert!(WorkerResultV1::parse_json(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut stale_candidate = result.clone();
    stale_candidate.candidate_identity.insert(
        "candidate_id".to_owned(),
        serde_json::json!(OTHER_CANDIDATE_ID),
    );
    assert!(stale_candidate.validate_for(&lease, &package).is_err());

    let mut missing_artifact = result.clone();
    missing_artifact.artifacts.remove(0);
    assert!(
        !r3_artifact_set_is_exact(&missing_artifact),
        "composed exact-set validation rejects a missing artifact row"
    );

    let mut replaced_artifact = result.clone();
    replaced_artifact.artifacts[0].path = R3_RESULT_PATH.to_owned();
    assert!(
        !r3_artifact_set_is_exact(&replaced_artifact),
        "composed exact-set validation rejects an allowed-path substitution"
    );

    let mut substituted_artifact = result.clone();
    substituted_artifact.artifacts[0].sha256 = sha(b"substituted artifact receipt");
    assert!(
        workspace
            .verify(&substituted_artifact, &lease, &package)
            .is_err()
    );

    let mut substituted_fixture = result.clone();
    let fixture = substituted_fixture
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.path == "fixtures/routine-production-catalog/cases.json")
        .unwrap();
    fixture.sha256 = sha(b"substituted fixture receipt");
    assert!(
        workspace
            .verify(&substituted_fixture, &lease, &package)
            .is_err()
    );
    eprintln!("ROUTINE_PRODUCTION_CATALOG_R3_RESULT_ID={result_id}");
}

fn r3_artifact_set_is_exact(result: &WorkerResultV1) -> bool {
    let expected_paths = BTreeSet::from([
        "fixtures/routine-production-catalog/cases.json",
        "validator/src/routine_work/catalog.rs",
        "validator/tests/routine_production_catalog_contract.rs",
    ]);
    let actual_paths = result
        .artifacts
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    let declared_count = result.candidate_identity["artifact_count"].as_u64();
    let declared_bytes = result.candidate_identity["artifact_bytes"].as_u64();
    let declared_aggregate = result.candidate_identity["corrected_artifact_set_sha256"].as_str();
    let actual_bytes = result
        .artifacts
        .iter()
        .map(|row| row.byte_length)
        .sum::<u64>();
    let mut aggregate_rows = result
        .artifacts
        .iter()
        .map(|row| {
            format!(
                "{}\t{}\n",
                row.path,
                row.sha256.strip_prefix("sha256:").unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    aggregate_rows.sort();
    let actual_aggregate = sha(aggregate_rows.concat().as_bytes());

    actual_paths == expected_paths
        && declared_count == Some(3)
        && result.artifacts.len() == 3
        && declared_bytes == Some(actual_bytes)
        && declared_aggregate == Some(actual_aggregate.as_str())
}

fn corrective_lease() -> (WorkPackage, LeaseSpec, ScopePolicy) {
    let owned_paths = canonical_paths(&[
        R3_RESULT_PATH,
        "validator/src/routine_work/catalog.rs",
        "validator/tests/routine_production_catalog_contract.rs",
    ]);
    let fixtures = canonical_paths(&["fixtures/routine-production-catalog/cases.json"]);
    let semantic_symbols = BTreeSet::from(["routine-work::production-catalog".to_owned()]);
    let effects = BTreeSet::from([
        EffectGrant::new(
            EffectClass::WorkspaceWrite,
            "routine-production-catalog-source",
        )
        .unwrap(),
        EffectGrant::new(
            EffectClass::FixtureWrite,
            "routine-production-catalog-fixtures",
        )
        .unwrap(),
        EffectGrant::new(EffectClass::Process, "offline-validation").unwrap(),
    ]);
    let owned_scope = OwnedScope {
        paths: owned_paths.clone(),
        semantic_symbols: semantic_symbols.clone(),
        generated_outputs: BTreeSet::new(),
        fixtures: fixtures.clone(),
        effects: effects.clone(),
    };
    let dependencies = BTreeSet::from([
        "accepted-routine-public-execution-mediator".to_owned(),
        "generated-impact-node-catalog-authority".to_owned(),
    ]);
    let package = WorkPackage {
        node_id: "routine-production-catalog".to_owned(),
        dependencies,
        required_tools: BTreeSet::from([
            "cargo-nextest".to_owned(),
            "rustfmt".to_owned(),
            "shasum".to_owned(),
        ]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope.clone(),
        prerequisites: BTreeSet::from([
            "preserve-prior-r2-candidate".to_owned(),
            "root-retains-public-dispatch-and-effect-grant-authority".to_owned(),
        ]),
        outputs: BTreeSet::from([
            "catalog-bound-symbolic-runner-authority".to_owned(),
            "typed-worker-result-receipt".to_owned(),
        ]),
        acceptance: BTreeSet::from([
            "caller-consistent-executable-substitution-refused".to_owned(),
            "worker-result-artifacts-verified".to_owned(),
        ]),
        claim_effect: "none".to_owned(),
    };
    let lease = LeaseSpec {
        lease_id: "ROUTINE-PRODUCTION-CATALOG-073-R3".to_owned(),
        run_id: "ultragoal-successor-live-20260713-routine-production-catalog-r3".to_owned(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse("/root/routine_catalog_single_spelling_engineer").unwrap(),
        binding: Binding::new(R3_CONTEXT_ID, R3_CANDIDATE_ID).unwrap(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 1_000_000,
        max_retries: 2,
    };
    let policy = ScopePolicy {
        allowed_paths: owned_paths,
        allowed_semantic_prefixes: semantic_symbols,
        allowed_fixtures: fixtures,
        allowed_effects: effects,
        ..ScopePolicy::default()
    };
    (package, lease, policy)
}

fn canonical_paths(values: &[&str]) -> BTreeSet<CanonicalPath> {
    values
        .iter()
        .map(|value| CanonicalPath::parse(value).unwrap())
        .collect()
}
