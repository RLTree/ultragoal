use super::scenario::*;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg(unix)]
#[test]
fn legacy_guidance_is_zero_write_bounded_and_never_executes_legacy_effects() {
    let repository = Repository::new("legacy-guidance-zero-write");
    for args in [
        &[
            "--json",
            "routine",
            "check",
            "--receipt",
            "legacy-marker.json",
        ][..],
        &["--json", "archive", "--receipt", "legacy-archive.json"][..],
        &["--json", "gc", "apply", "--receipt", "legacy-gc.json"][..],
        &[
            "--json",
            "transaction",
            "finalize",
            "--receipt",
            "legacy-final.json",
        ][..],
        &[
            "--json",
            "openai",
            "call",
            "prove",
            "--receipt",
            "legacy-call.json",
        ][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert_eq!(output.status.code(), Some(4), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            value["schema_version"],
            "harness-ultragoal.compatibility-guidance.v1"
        );
        assert_eq!(value["disposition"], "guidance_only");
        assert_eq!(value["legacy_effect_executed"], false);
        assert_eq!(observe(&repository.root), before, "{args:?}");
    }
}

#[cfg(unix)]
#[test]
fn legacy_help_guidance_does_not_echo_untrusted_tail_or_absolute_root() {
    let repository = Repository::new("legacy-guidance-redaction");
    let before = observe(&repository.root);
    let output = repository.run(&[
        "--json",
        "current-state",
        "--help",
        "NEVER_ECHO_LEGACY_GUIDANCE_7124",
    ]);
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    let bytes = emitted(&output);
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("NEVER_ECHO_LEGACY_GUIDANCE_7124"));
    assert!(!text.contains(repository.root.to_str().unwrap()));
    assert_eq!(observe(&repository.root), before);
}

#[cfg(unix)]
#[test]
fn human_legacy_guidance_is_bounded_zero_write_and_non_effectful() {
    let repository = Repository::new("legacy-human-guidance");
    let before = observe(&repository.root);
    let output = repository.run(&["archive", "--receipt", "NEVER_ECHO_HUMAN_GUIDANCE_6198"]);
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).expect("human compatibility guidance");
    assert!(text.contains("CLI_LEGACY_ROUTE_GUIDANCE"));
    assert!(text.contains("canonical target: package/migrate"));
    assert!(text.contains("no legacy effect was executed"));
    assert!(!text.contains("NEVER_ECHO_HUMAN_GUIDANCE_6198"));
    assert!(!text.contains(repository.root.to_str().unwrap()));
    assert_eq!(observe(&repository.root), before);
}

#[cfg(unix)]
#[test]
fn non_utf8_input_is_a_typed_zero_write_error_without_byte_echo() {
    let repository = Repository::new("non-utf8-typed-error");
    let before = observe(&repository.root);
    let private = std::ffi::OsString::from_vec(vec![0xff]);
    for args in [
        vec![
            std::ffi::OsString::from("--json"),
            private.clone(),
            std::ffi::OsString::from("archive"),
        ],
        vec![
            private.clone(),
            std::ffi::OsString::from("archive"),
            std::ffi::OsString::from("--json"),
        ],
    ] {
        let output = repository.run(args);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["schema_version"], "harness-ultragoal.cli-error.v1");
        assert_eq!(value["error_id"], "CLI_NON_UTF8_ARGUMENT");
        assert_eq!(value["exit_code"], 2);
        assert_eq!(
            output.stderr,
            b"{\"schema_version\":\"harness-ultragoal.cli-error.v1\",\"error_id\":\"CLI_NON_UTF8_ARGUMENT\",\"exit_code\":2}\n"
        );
    }
    assert_eq!(observe(&repository.root), before);
}
