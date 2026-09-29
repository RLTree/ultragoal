use super::*;
use crate::repository_fit::product_adapter::{AdapterErrorId, FitPlanScope, plan_target_for_scope};

#[test]
pub(crate) fn deleted_planned_template_paths_refuse_fit_plan_without_effect() {
    for disposition in ["unstaged", "staged", "renamed"] {
        let fixture = Fixture::new(&format!("tracked-deletion-{disposition}"));
        commit_template(&fixture, "AGENTS.md");
        match disposition {
            "unstaged" => fs::remove_file(fixture.root.join("AGENTS.md")).unwrap(),
            "staged" => {
                fs::remove_file(fixture.root.join("AGENTS.md")).unwrap();
                git(&fixture.root, &["add", "--update"]);
            }
            "renamed" => {
                fs::rename(
                    fixture.root.join("AGENTS.md"),
                    fixture.root.join("operator-instructions.md"),
                )
                .unwrap();
                git(&fixture.root, &["add", "--all"]);
            }
            _ => unreachable!(),
        }
        let context = fixture.context();
        let failure = assert_zero_write(&fixture, || plan_target(&context).unwrap_err());
        assert_eq!(failure.id(), AdapterErrorId::PlanConflict, "{disposition}");
    }
}

#[test]
pub(crate) fn deleted_unrelated_path_does_not_block_fit_plan() {
    let fixture = Fixture::new("unrelated-tracked-deletion");
    fixture.write("operator-notes.txt", b"keep deletion\n");
    commit(&fixture);
    fs::remove_file(fixture.root.join("operator-notes.txt")).unwrap();

    let record = assert_zero_write(&fixture, || plan_target(&fixture.context()).unwrap());
    assert_eq!(record.conflict_count(), 0);
}

#[test]
pub(crate) fn deleted_routine_configuration_blocks_only_the_routine_scope() {
    let fixture = Fixture::new("routine-config-tracked-deletion");
    commit_template(&fixture, "config/routine-public.json");
    fs::remove_file(fixture.root.join("config/routine-public.json")).unwrap();

    let context = fixture.context();
    let failure = assert_zero_write(&fixture, || {
        plan_target_for_scope(&context, FitPlanScope::RoutineConfiguration).unwrap_err()
    });
    assert_eq!(failure.id(), AdapterErrorId::PlanConflict);
}

fn commit_template(fixture: &Fixture, path: &str) {
    fixture.write_template(path);
    commit(fixture);
}

fn commit(fixture: &Fixture) {
    git(
        &fixture.root,
        &["config", "user.email", "fit@example.invalid"],
    );
    git(&fixture.root, &["config", "user.name", "Repository Fit"]);
    git(&fixture.root, &["add", "--all"]);
    git(&fixture.root, &["commit", "--quiet", "-m", "user state"]);
}
