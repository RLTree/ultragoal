use super::repository_fit::{
    CanonicalPath, DesiredFile, DesiredState, FitErrorId, FitMode, Ownership, PlanAuthorization,
    inspect, plan,
};
use super::support::{MemoryRepo, desired, file, sha};

#[test]
fn paths_reject_escape_alias_ambiguity_controls_and_unbounded_depth() {
    for raw in [
        "",
        "/absolute",
        "../escape",
        "a/../escape",
        "./a",
        "a//b",
        "a/",
        "a\\b",
        "a\0b",
        "unicodé",
        "a/a/a/a/a/a/a/a/a/a/a/a/a/a/a/a/a",
    ] {
        assert_eq!(
            CanonicalPath::parse(raw).unwrap_err().id(),
            FitErrorId::InvalidPath,
            "{raw:?}"
        );
    }
    let duplicate = DesiredState::new(
        sha(b'a'),
        sha(b'b'),
        vec![
            file("Agents.md", b"one", Ownership::UserOwned, &[]),
            file("AGENTS.md", b"two", Ownership::UserOwned, &[]),
        ],
    )
    .unwrap_err();
    assert_eq!(duplicate.id(), FitErrorId::InvalidSpec);
}

#[test]
fn desired_state_rejects_empty_invalid_digests_prior_rows_and_large_objects() {
    assert_eq!(
        DesiredState::new("invalid".into(), sha(b'b'), vec![])
            .unwrap_err()
            .id(),
        FitErrorId::InvalidSpec
    );
    assert_eq!(
        DesiredState::new(sha(b'a').to_ascii_uppercase(), sha(b'b'), vec![])
            .unwrap_err()
            .id(),
        FitErrorId::InvalidSpec
    );
    assert_eq!(
        DesiredFile::managed(
            CanonicalPath::parse("A.md").unwrap(),
            vec![],
            sha(b'a'),
            sha(b'b'),
            sha(b'f'),
            sha(b'e'),
            ["not-a-digest".into()],
        )
        .unwrap_err()
        .id(),
        FitErrorId::InvalidSpec
    );
    assert_eq!(
        DesiredFile::user_owned(
            CanonicalPath::parse("A.md").unwrap(),
            vec![0; 4 * 1024 * 1024 + 1],
        )
        .unwrap_err()
        .id(),
        FitErrorId::ResourceLimit
    );
}

#[test]
fn desired_bytes_never_appear_in_debug_or_stable_errors() {
    let secret = "do-not-project-this-content";
    let file = file("A.md", secret.as_bytes(), Ownership::HarnessGenerated, &[]);
    assert!(!format!("{file:?}").contains(secret));
    let error = CanonicalPath::parse("../do-not-project-this-path").unwrap_err();
    assert!(!format!("{error:?}").contains("do-not-project"));
    assert!(!error.to_string().contains("do-not-project"));
}

#[test]
fn inspection_and_plan_are_exactly_desired_state_and_mode_bound() {
    let first = desired(vec![file("A.md", b"a", Ownership::UserOwned, &[])]);
    let second = desired(vec![file("A.md", b"b", Ownership::UserOwned, &[])]);
    let mut repo = MemoryRepo::default();
    let fresh = inspect(FitMode::Fresh, &first, &mut repo).unwrap();
    assert_eq!(
        plan(&fresh, &second).unwrap_err().id(),
        FitErrorId::StaleBinding
    );
    let fresh_plan = plan(&fresh, &first).unwrap();
    let retrofit_plan = plan(
        &inspect(FitMode::Retrofit, &first, &mut repo).unwrap(),
        &first,
    )
    .unwrap();
    assert_ne!(fresh_plan.plan_sha256(), retrofit_plan.plan_sha256());
}

#[test]
fn authorization_rejects_malformed_or_uppercase_binding_digests() {
    for value in ["bad".to_string(), sha(b'a').to_ascii_uppercase()] {
        assert_eq!(
            PlanAuthorization::new(sha(b'a'), sha(b'b'), value)
                .unwrap_err()
                .id(),
            FitErrorId::InvalidSpec
        );
    }
}
