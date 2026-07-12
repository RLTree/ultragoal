use crate::retained_routes::{DigestEvidence, EntryEvidence, VerificationError};
use crate::support::Fixture;

#[test]
fn source_identity_kind_digest_state_and_status_tamper_fail_closed() {
    let baseline = Fixture::baseline().sources[0];
    let wrong = [
        EntryEvidence {
            path: "validator/src/argument_parser/wrong.rs",
            ..baseline
        },
        EntryEvidence {
            stable_id: "LEGACY-COMMAND:wrong",
            ..baseline
        },
        EntryEvidence {
            kind: "legacy-lane-authority",
            ..baseline
        },
        EntryEvidence {
            sha256: "00",
            ..baseline
        },
        EntryEvidence {
            authority_state: "canonical",
            ..baseline
        },
        EntryEvidence {
            active_status: "context-only",
            ..baseline
        },
    ];
    for source in wrong {
        let mut fixture = Fixture::baseline();
        fixture.sources[0] = source;
        let expected = if source.stable_id != baseline.stable_id
            || source.path != baseline.path
            || source.kind != baseline.kind
        {
            VerificationError::UnregisteredSource
        } else {
            VerificationError::SourceMismatch
        };
        assert_eq!(fixture.verify(), Err(expected));
    }
}

#[test]
fn current_bytes_and_complete_source_set_are_required() {
    let mut stale = Fixture::baseline();
    stale.digests[0].sha256 = "00";
    assert_eq!(stale.verify(), Err(VerificationError::SourceBytesMismatch));

    let mut missing_digest = Fixture::baseline();
    missing_digest.digests.pop();
    assert_eq!(
        missing_digest.verify(),
        Err(VerificationError::DigestSetMismatch)
    );

    let mut extra_digest = Fixture::baseline();
    extra_digest.digests.push(DigestEvidence {
        path: "unregistered/path.rs",
        sha256: "00",
    });
    assert_eq!(
        extra_digest.verify(),
        Err(VerificationError::DigestSetMismatch)
    );

    let mut missing_source = Fixture::baseline();
    missing_source.sources.pop();
    assert_eq!(
        missing_source.verify(),
        Err(VerificationError::SourceSetMismatch)
    );
}

#[test]
fn newly_discovered_unregistered_source_fails_closed() {
    let mut fixture = Fixture::baseline();
    fixture.sources.push(EntryEvidence {
        stable_id: "LEGACY-COMMAND:validator/src/new_command.rs",
        kind: "legacy-command-authority",
        path: "validator/src/new_command.rs",
        sha256: "00",
        authority_state: "legacy",
        active_status: "active",
    });
    assert_eq!(fixture.verify(), Err(VerificationError::UnregisteredSource));
}
