use super::scenario::*;
use std::collections::BTreeSet;

#[cfg(unix)]
#[test]
fn public_catalog_is_sole_and_legacy_routes_fail_closed_without_writes() {
    let repository = Repository::new("sole-catalog");
    let initial = observe(&repository.root);
    assert!(!initial.status.is_empty(), "fixture must be dirty");

    for args in [
        &["--json", PRIVATE_CANARY][..],
        &["--json", "help"][..],
        &["--json", "current-state", "--help"][..],
        &["--json", "observe", "logs", "query", "--help"][..],
        &["--json", "package", "digest", "--help"][..],
        &["--json", "source", "audit"][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert_machine_error(&output);
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }

    let help = repository.run(&["--json", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(help.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&help.stdout).expect("help JSON");
    let groups = value["commands"]
        .as_array()
        .expect("command catalog")
        .iter()
        .filter_map(|row| row["group"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        groups,
        BTreeSet::from([
            "check", "diagnose", "eval", "fit", "inspect", "migrate", "next", "observe", "package",
            "prove"
        ])
    );
    assert_eq!(observe(&repository.root), initial);
}

#[cfg(unix)]
#[test]
fn public_read_help_parse_and_query_paths_are_recursively_zero_write() {
    let repository = Repository::new("read-zero-write");
    let initial = observe(&repository.root);
    for args in [
        &["--json", "--version"][..],
        &["--json", "inspect", "context"][..],
        &["--json", "inspect", "inventory"][..],
        &["--json", "inspect"][..],
        &["--json", "next"][..],
        &["--json", "diagnose"][..],
        &["--json", "observe", "query"][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert!(
            matches!(output.status.code(), Some(0 | 1 | 3 | 4)),
            "{args:?}: {output:?}"
        );
        let value: serde_json::Value =
            serde_json::from_slice(&emitted(&output)).expect("versioned JSON output");
        assert!(
            value["schema_version"]
                .as_str()
                .is_some_and(|schema| schema.ends_with("-v1") || schema.ends_with(".v1")),
            "{args:?}: {value}"
        );
        assert!(!String::from_utf8_lossy(&emitted(&output)).contains(PRIVATE_CANARY));
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }
    assert_eq!(observe(&repository.root), initial);
}
