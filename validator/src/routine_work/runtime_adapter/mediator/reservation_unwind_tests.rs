use super::terminal_settlement_fixture::*;
use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

fn staged_fixture(label: &str) -> (PathBuf, StagedProgram) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = manifest
        .parent()
        .expect("reservation fixture manifest has no workspace parent")
        .join("target/routine-reservation-lifecycle-fixtures");
    fs::create_dir_all(&parent).expect("reservation fixture parent is unavailable");
    let directory = parent.join(format!(
        "{label}-{}-{}",
        std::process::id(),
        NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let program = directory.join("program");
    let marker = directory.join("authority");
    let seal = directory.join("seal");
    fs::copy("/usr/bin/true", &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o555)).unwrap();
    let marker_bytes = b"reservation-unwind-marker\n".to_vec();
    let seal_bytes = b"reservation-unwind-seal\n".to_vec();
    fs::write(&marker, &marker_bytes).unwrap();
    fs::write(&seal, &seal_bytes).unwrap();
    let executable = PinnedExecutable::open_unbound(&program).unwrap();
    let staged = StagedProgram {
        executable,
        directory: directory.clone(),
        marker: marker.clone(),
        seal: seal.clone(),
        marker_bytes,
        seal_bytes,
        directory_identity: ObjectIdentity::from(&fs::metadata(&directory).unwrap()),
        marker_identity: ObjectIdentity::from(&fs::metadata(marker).unwrap()),
        seal_identity: ObjectIdentity::from(&fs::metadata(seal).unwrap()),
    };
    (directory, staged)
}

#[test]
fn cleanup_failures_never_replace_the_initiating_panic_or_erase_recovery() {
    for (label, cleanup_cause) in [
        ("directory", "routine-production-launch-directory-mismatch"),
        ("entry", "routine-production-launch-entry-mismatch"),
        ("missing", "routine-production-launch-entry-missing"),
    ] {
        let durable = Arc::new(TerminalDurable::default());
        *durable
            .cleanup_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(cleanup_cause);
        let reservation = attempt(label, Some(durable.clone()), false, None);
        let protocol = reservation.protocol_id.clone();
        let grant = reservation.grant_id.clone();
        let marker = reservation.recovery_marker.clone();
        let foreign_protocol = format!("{protocol}-foreign");
        let (stage_root, staged) = staged_fixture(label);
        reservation.staged.borrow_mut().push(staged);
        {
            let mut state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.active_protocols.insert(protocol.clone(), grant);
            state
                .ambiguous_protocols
                .insert(foreign_protocol.clone(), "foreign-marker".to_owned());
        }

        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), RoutineError> = run_reserved(reservation, |attempt| {
                attempt.mark_started()?;
                panic!("reservation-unwind-original-payload");
            });
        }));
        let payload = unwound.unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&'static str>().copied(),
            Some("reservation-unwind-original-payload")
        );
        assert_eq!(durable.cleanup_calls.load(Ordering::SeqCst), 1);
        assert!(stage_root.is_dir());
        {
            let state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert!(!state.active_protocols.contains_key(&protocol));
            assert_eq!(state.ambiguous_protocols.get(&protocol), Some(&marker));
            assert_eq!(
                state
                    .ambiguous_protocols
                    .get(&foreign_protocol)
                    .map(String::as_str),
                Some("foreign-marker")
            );
        }

        let recovery_durable = Arc::new(TerminalDurable::default());
        let mut recovery = retry_grant(&attempt(label, None, false, None), recovery_durable);
        recovery.protocol_id.clone_from(&protocol);
        recovery.recovery_for = Some(marker);
        let retry = reserve_grant(&recovery).unwrap();
        retry.settle_incomplete(DurableSettlement::Failed).unwrap();
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!state.active_protocols.contains_key(&protocol));
        assert!(!state.ambiguous_protocols.contains_key(&protocol));
        assert_eq!(
            state
                .ambiguous_protocols
                .get(&foreign_protocol)
                .map(String::as_str),
            Some("foreign-marker")
        );
        state.ambiguous_protocols.remove(&foreign_protocol);
        drop(state);
        fs::remove_dir_all(stage_root).unwrap();
    }
}
