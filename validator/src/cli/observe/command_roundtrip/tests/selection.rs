use super::super::requested_specs;
use super::command;

#[test]
fn requested_specs_fail_closed_for_ambiguous_or_unknown_targets() {
    let package = requested_specs(&command()).expect("package spec");
    assert_eq!(package[0].id, "package digest");

    let mut family = command();
    family.target_command = None;
    family.target_family = Some("package".to_string());
    assert_eq!(
        requested_specs(&family).expect("family")[0].family,
        "package"
    );

    let mut both = family;
    both.target_command = Some("package digest".to_string());
    assert!(
        requested_specs(&both)
            .expect_err("ambiguous")
            .contains("either")
    );

    let mut missing = command();
    missing.target_command = None;
    assert!(
        requested_specs(&missing)
            .expect_err("missing")
            .contains("requires")
    );

    let mut unknown_family = command();
    unknown_family.target_command = None;
    unknown_family.target_family = Some("unknown-family".to_string());
    assert!(
        requested_specs(&unknown_family)
            .expect_err("unknown family")
            .contains("unknown observability family spec")
    );
}
