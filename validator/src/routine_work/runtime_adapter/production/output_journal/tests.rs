use super::super::ledger::{AttemptState, AuthorityBinding, ReservationSpec};
use super::apply::{ApplyEvent, apply_observed};
use super::*;
use crate::routine_work::digest::sha256;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    workspace: PathBuf,
    authority: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let parent =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/routine-output-journal-tests");
        fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let workspace = root.join("workspace");
        let authority = root.join("authority");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&authority).unwrap();
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&authority, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root,
            workspace,
            authority,
        }
    }

    fn teardown(self) {
        fs::remove_dir_all(&self.root).unwrap();
        assert!(!self.root.exists());
    }
}

fn digest(label: &str) -> String {
    sha256(format!("routine-output-journal-test-{label}").as_bytes())
}

fn binding(label: &str) -> AuthorityBinding {
    AuthorityBinding {
        protocol_id: digest(&format!("{label}-protocol")),
        effect_id: digest(&format!("{label}-effect")),
        context_id: digest(&format!("{label}-context")),
        candidate_id: digest(&format!("{label}-candidate")),
        plan_id: digest(&format!("{label}-plan")),
        snapshot_id: digest(&format!("{label}-snapshot")),
    }
}

fn reserve(
    ledger: &FileAuthorityLedger,
    label: &str,
    journal: OutputProvisionJournal,
    recovery_for: Option<String>,
) -> ReservationToken {
    ledger
        .reserve(ReservationSpec {
            binding: binding(label),
            request_id: digest(&format!("{label}-request")),
            grant_id: digest(&format!("{label}-grant-{recovery_for:?}")),
            recovery_marker: digest(&format!("{label}-recovery-{recovery_for:?}")),
            recovery_for,
            reuse_only: false,
            reuse_preauthorization: None,
            output_journal: journal,
        })
        .unwrap()
}

fn scope() -> RepoPath {
    RepoPath::parse("target/routine/compile").unwrap()
}

#[test]
fn every_created_and_recorded_boundary_recovers_from_the_exact_durable_journal() {
    for (ordinal, boundary) in [
        (0, ("target", false)),
        (1, ("target", true)),
        (2, ("target/routine", false)),
        (3, ("target/routine", true)),
        (4, ("target/routine/compile", false)),
        (5, ("target/routine/compile", true)),
    ] {
        let label = format!("boundary-{ordinal}");
        let fixture = Fixture::new(&label);
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let first = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
        );
        let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
            let matches = match event {
                ApplyEvent::Created(path) => path == boundary.0 && !boundary.1,
                ApplyEvent::Recorded(path) => path == boundary.0 && boundary.1,
            };
            if matches {
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

#[test]
fn recovery_preserves_foreign_content_and_keeps_the_attempt_pending() {
    let fixture = Fixture::new("foreign-content");
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let first = reserve(
        &ledger,
        "foreign-content",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
    );
    let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
        if matches!(event, ApplyEvent::Created("target/routine/compile")) {
            Err(error("routine-output-journal-test-interruption"))
        } else {
            Ok(())
        }
    });
    assert!(interrupted.is_err());
    let foreign = fixture.workspace.join("target/routine/compile/foreign");
    fs::write(&foreign, b"foreign").unwrap();
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    let recovered = reserve(
        &ledger,
        "foreign-content",
        pending.output_journal,
        Some(pending.marker),
    );
    let failure = apply(&ledger, &recovered, &fixture.workspace).unwrap_err();
    assert_eq!(
        failure.cause(),
        "routine-production-output-owned-content-ambiguous"
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"foreign");
    assert!(
        ledger
            .pending_recovery(&recovered.binding)
            .unwrap()
            .is_some()
    );
    drop(ledger);
    fixture.teardown();
}

#[test]
fn preexisting_exact_scope_is_read_only_and_symlink_prefix_is_refused() {
    let fixture = Fixture::new("preexisting");
    fs::create_dir_all(fixture.workspace.join("target/routine/compile")).unwrap();
    let before = fs::symlink_metadata(fixture.workspace.join("target/routine/compile")).unwrap();
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let token = reserve(
        &ledger,
        "preexisting",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
    );
    apply(&ledger, &token, &fixture.workspace).unwrap();
    let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
    assert!(
        pending
            .output_journal
            .components
            .iter()
            .all(|component| component.provisioned.is_none())
    );
    let after = fs::symlink_metadata(fixture.workspace.join("target/routine/compile")).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    ledger
        .settle(&token, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    drop(ledger);

    fs::remove_dir_all(fixture.workspace.join("target")).unwrap();
    let outside = fixture.root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(fixture.workspace.join("target")).unwrap();
    symlink(&outside, fixture.workspace.join("target/routine")).unwrap();
    assert!(observe(&fixture.workspace, &[scope()]).is_err());
    fixture.teardown();
}
