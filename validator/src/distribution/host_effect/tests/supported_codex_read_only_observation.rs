#[test]
fn supported_host_oracle_is_closed_before_selected_executable_or_process_entrypoints() {
    let oracle = include_str!("../../../cli/successor_public/supported_host_oracle.rs");
    assert!(oracle.contains("pub(super) const fn is_available() -> bool"));
    assert!(oracle.contains("false"));
    for forbidden in [
        "observe_supported_read_only",
        "std::process::Command",
        "run_bounded",
        "plugin list",
        "plugin marketplace list",
    ] {
        assert!(
            !oracle.contains(forbidden),
            "closed supported-host gate retained process surface {forbidden}"
        );
    }

    let selected = include_str!("../selected_codex_executable.rs");
    assert!(!selected.contains("mod read_only_observation"));
}

#[test]
fn supported_host_handoff_has_no_path_derived_codex_version() {
    let source = include_str!("../../../cli/successor_public/package_personal_install.rs");
    assert!(!source.contains("selected_version_from_path"));
    let plan = source
        .split_once("pub(super) fn plan(")
        .unwrap()
        .1
        .split_once("pub(super) fn verify(")
        .unwrap()
        .0;
    assert!(
        plan.find("supported_host_oracle::is_available").unwrap()
            < plan.find("build_handoff(").unwrap()
    );
}
