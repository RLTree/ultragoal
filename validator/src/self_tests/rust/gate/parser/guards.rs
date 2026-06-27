use crate::cli::rust::parse as parse_rust;

#[test]
fn rust_parser_rejects_partial_subcommand_substitutes() {
    for args in [
        ["rust", "toolchain", "wrong"].as_slice(),
        ["rust", "memory", "wrong"].as_slice(),
        ["rust", "dependency", "wrong"].as_slice(),
        ["rust", "coverage", "wrong"].as_slice(),
        ["rust", "workspace", "topology", "wrong"].as_slice(),
    ] {
        assert!(
            parse_rust(
                &args
                    .iter()
                    .map(|arg| (*arg).to_string())
                    .collect::<Vec<_>>()
            )
            .is_err(),
            "{args:?}"
        );
    }
}
