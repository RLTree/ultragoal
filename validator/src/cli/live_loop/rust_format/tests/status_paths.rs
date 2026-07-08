use super::super::{changed_paths_from_status, is_rust_source, is_rustfmt_config};

#[test]
fn status_parser_selects_changed_rust_and_skips_deletes() {
    let paths = changed_paths_from_status(
        " M validator/src/cli/live_loop/mod.rs\n M validator/tests/cli_surface.rs\n D validator/src/old.rs\n?? docs/note.md\nR  old.rs -> validator/src/new.rs\n",
    );
    let rust_paths = paths
        .iter()
        .filter(|path| is_rust_source(path))
        .collect::<Vec<_>>();
    assert_eq!(rust_paths.len(), 3);
    assert!(rust_paths.iter().any(|path| path.ends_with("mod.rs")));
    assert!(
        rust_paths
            .iter()
            .any(|path| path.ends_with("tests/cli_surface.rs"))
    );
    assert!(rust_paths.iter().any(|path| path.ends_with("new.rs")));
}

#[test]
fn empty_status_reports_no_changed_paths() {
    assert!(changed_paths_from_status("\n  \n").is_empty());
}

#[test]
fn porcelain_z_status_parser_keeps_spaces_and_rename_sources_visible() {
    let paths = changed_paths_from_status(
        "?? validator/src/foo bar.rs\0R  validator/src/new name.rs\0validator/src/old name.rs\0 D validator/src/deleted name.rs\0 D validator/rustfmt.toml\0",
    );
    let rust_paths = paths
        .iter()
        .filter(|path| is_rust_source(path))
        .collect::<Vec<_>>();

    assert!(
        rust_paths
            .iter()
            .any(|path| path == &&std::path::PathBuf::from("validator/src/foo bar.rs")),
        "porcelain -z paths must not keep human status quotes"
    );
    assert!(rust_paths.iter().any(|path| path.ends_with("new name.rs")));
    assert!(rust_paths.iter().any(|path| path.ends_with("old name.rs")));
    assert!(
        !rust_paths
            .iter()
            .any(|path| path.ends_with("deleted name.rs")),
        "deleted source files are not direct rustfmt inputs"
    );
    assert!(
        paths.iter().any(|path| is_rustfmt_config(path)),
        "deleted rustfmt config still forces workspace formatting"
    );
}

#[test]
fn rustfmt_config_promotes_to_workspace_format_plan() {
    for config in [
        "rustfmt.toml",
        ".rustfmt.toml",
        "validator/rustfmt.toml",
        "validator/.rustfmt.toml",
    ] {
        let paths = changed_paths_from_status(&format!(" M {config}\n M validator/src/lib.rs\n"));
        assert!(
            paths.iter().any(|path| is_rustfmt_config(path)),
            "{config} should force workspace formatting"
        );
    }
}

#[test]
fn deleted_rustfmt_config_still_promotes_to_workspace_format_plan() {
    let paths = changed_paths_from_status(" D validator/rustfmt.toml\n D validator/src/old.rs\n");
    assert!(
        paths.iter().any(|path| is_rustfmt_config(path)),
        "deleted rustfmt config must force workspace formatting"
    );
    assert!(
        !paths.iter().any(|path| path.ends_with("old.rs")),
        "deleted Rust source files are not direct rustfmt inputs"
    );
}

#[test]
fn renamed_rustfmt_config_still_promotes_to_workspace_format_plan() {
    let paths = changed_paths_from_status("R  validator/rustfmt.toml -> docs/rustfmt.old\n");
    assert!(
        paths.iter().any(|path| is_rustfmt_config(path)),
        "renamed-away rustfmt config must force workspace formatting"
    );
}
