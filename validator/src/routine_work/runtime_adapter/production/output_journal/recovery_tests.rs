use super::super::ledger::AttemptState;
use super::apply::{ApplyEvent, apply_observed};
use super::tests::{Fixture, reserve, scope};
use super::*;

const COMPONENTS: [&str; 3] = ["target", "target/routine", "target/routine/compile"];

#[test]
fn recorded_stage_and_publish_boundaries_recover_exactly() {
    for (ordinal, (path, boundary)) in COMPONENTS
        .into_iter()
        .flat_map(|path| [1, 2, 3].map(move |boundary| (path, boundary)))
        .enumerate()
    {
        let label = format!("recorded-boundary-{ordinal}");
        let fixture = Fixture::new(&label);
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let first = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
        );
        let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
            if boundary_matches(event, path, boundary) {
                Err(error("routine-output-journal-test-interruption"))
            } else {
                Ok(())
            }
        });
        assert!(interrupted.is_err());
        let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
        drop(ledger);

        let ledger = FileAuthorityLedger::open_existing(&fixture.authority).unwrap();
        let recovered = reserve(
            &ledger,
            &label,
            pending.output_journal,
            Some(pending.marker),
        );
        apply(&ledger, &recovered, &fixture.workspace).unwrap();
        assert!(fixture.workspace.join("target/routine/compile").is_dir());
        super::custody_tests::assert_no_staged_outputs(&fixture.workspace);
        ledger
            .settle(&recovered, AttemptState::Failed, &BTreeMap::new())
            .unwrap();
        assert!(
            ledger
                .pending_recovery(&recovered.binding)
                .unwrap()
                .is_none()
        );
        drop(ledger);
        fixture.teardown();
    }
}

fn boundary_matches(event: ApplyEvent<'_>, path: &str, boundary: u8) -> bool {
    matches!(
        (event, boundary),
        (ApplyEvent::StageRecorded(observed), 1)
            | (ApplyEvent::Published(observed), 2)
            | (ApplyEvent::FinalRecorded(observed), 3)
            if observed == path
    )
}
