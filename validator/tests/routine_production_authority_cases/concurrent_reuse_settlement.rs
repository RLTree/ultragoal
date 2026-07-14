use super::*;

#[test]
pub(crate) fn forged_and_valid_reuse_concurrency_is_order_independent_and_single_transition() {
    for forged_first in [true, false] {
        let repo = TempRepo::new(if forged_first {
            "production-forged-valid-race-forged-first"
        } else {
            "production-forged-valid-race-valid-first"
        });
        repo.write(".git/info/exclude", b"target/\n");
        fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 103 }\n");
        let fixture = fixture_from_repo(repo, "routine-production-race");
        let authority = AuthorityRoot::new(if forged_first {
            "production-forged-valid-race-forged-first"
        } else {
            "production-forged-valid-race-valid-first"
        });
        let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
        let valid_reuse = first.reuse_artifacts().to_vec();
        let forged_reuse = foreign_reuse_for_same_request(
            &fixture,
            if forged_first {
                "production-forged-valid-race-foreign-a"
            } else {
                "production-forged-valid-race-foreign-b"
            },
        );
        assert_ne!(valid_reuse[0], forged_reuse[0]);
        let valid_path = authority.parent.join("valid-reuse-artifact");
        let forged_path = authority.parent.join("forged-reuse-artifact");
        fs::write(&valid_path, &valid_reuse[0]).unwrap();
        fs::write(&forged_path, &forged_reuse[0]).unwrap();
        let before_target = fixture.repo.tree();
        let before_status = fixture.repo.status();

        let executable = std::env::current_exe().unwrap();
        let outcome_first = authority.parent.join("ordered-outcome-first");
        let outcome_second = authority.parent.join("ordered-outcome-second");
        let spawn = |reuse: &Path, outcome: &Path| {
            Command::new(&executable)
                .args([
                    "--ignored",
                    "--exact",
                    "production_child_race_attempt",
                    "--nocapture",
                ])
                .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
                .env("HUL_ROUTINE_REPO", fixture.repo.root())
                .env("HUL_ROUTINE_AUTHORITY", authority.path())
                .env("HUL_ROUTINE_OUTCOME", outcome)
                .env("HUL_ROUTINE_REUSE", reuse)
                .spawn()
                .unwrap()
        };
        let (first_path, second_path) = if forged_first {
            (&forged_path, &valid_path)
        } else {
            (&valid_path, &forged_path)
        };
        let mut first_child = spawn(first_path, &outcome_first);
        let mut second_child = spawn(second_path, &outcome_second);
        assert!(first_child.wait().unwrap().success());
        assert!(second_child.wait().unwrap().success());
        let outcomes = [
            fs::read_to_string(&outcome_first).unwrap(),
            fs::read_to_string(&outcome_second).unwrap(),
        ];
        assert_eq!(
            outcomes.iter().filter(|value| *value == "complete").count(),
            1,
            "outcomes={outcomes:?}"
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|value| *value == "mediator-production-reuse-not-authenticated")
                .count(),
            1,
            "outcomes={outcomes:?}"
        );
        assert_eq!(authority_cardinalities(&authority), (1, 1, 2));
        assert_eq!(fixture.repo.tree(), before_target);
        assert_eq!(fixture.repo.status(), before_status);
        let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
        assert!(
            reopened
                .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
                .unwrap()
                .is_none()
        );
        let exact = mediate(&authority, &fixture, prepared(&fixture), valid_reuse).unwrap();
        assert_eq!(
            exact.nodes()[0].disposition(),
            RoutineNodeDisposition::Reused
        );
    }
}

#[test]
pub(crate) fn concurrent_reuse_at_consumed_grant_capacity_publishes_one_valid_max_state() {
    let repo = TempRepo::new("production-capacity-process-race");
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 101 }\n");
    let fixture = fixture_from_repo(repo, "routine-production-race");
    let authority = AuthorityRoot::new("production-capacity-process-race");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse_path = authority.parent.join("reuse-artifact");
    fs::write(&reuse_path, &first.reuse_artifacts()[0]).unwrap();
    let (_, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer.test_seed_capacity(1, consumed_limit - 1).unwrap();
    drop(issuer);
    drop(first);

    let executable = std::env::current_exe().unwrap();
    let outcome_a = authority.parent.join("capacity-outcome-a");
    let outcome_b = authority.parent.join("capacity-outcome-b");
    let spawn = |outcome: &Path| {
        Command::new(&executable)
            .args([
                "--ignored",
                "--exact",
                "production_child_race_attempt",
                "--nocapture",
            ])
            .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
            .env("HUL_ROUTINE_REPO", fixture.repo.root())
            .env("HUL_ROUTINE_AUTHORITY", authority.path())
            .env("HUL_ROUTINE_OUTCOME", outcome)
            .env("HUL_ROUTINE_REUSE", &reuse_path)
            .spawn()
            .unwrap()
    };
    let mut child_a = spawn(&outcome_a);
    let mut child_b = spawn(&outcome_b);
    assert!(child_a.wait().unwrap().success());
    assert!(child_b.wait().unwrap().success());
    let outcomes = [
        fs::read_to_string(outcome_a).unwrap(),
        fs::read_to_string(outcome_b).unwrap(),
    ];
    assert_eq!(
        outcomes.iter().filter(|value| *value == "complete").count(),
        1,
        "outcomes={outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| *value == "routine-production-authority-capacity-exhausted")
            .count(),
        1,
        "outcomes={outcomes:?}"
    );
    assert_eq!(authority_cardinalities(&authority), (1, 1, consumed_limit));
    let max_state = authority_state(&authority);
    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    assert!(
        reopened
            .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
            .unwrap()
            .is_none()
    );
    let replay = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}

pub(crate) fn tree(root: &Path) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut rows);
    }
    rows
}

pub(crate) fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, String>) {
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
            visit(root, &path, rows);
        } else if metadata.file_type().is_symlink() {
            rows.insert(relative, "symlink".to_owned());
        } else if metadata.is_file() {
            rows.insert(relative, format!("file:{}", sha(&fs::read(&path).unwrap())));
        } else {
            rows.insert(relative, "special".to_owned());
        }
    }
}
