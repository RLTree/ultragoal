pub(super) fn typed_cli_command_boundary_text(rel: &str, text: &str) -> bool {
    typed_argument_parser_boundary(rel, text)
        || impacted_rust_tests_parser_boundary(rel, text)
        || typed_execution_projection_boundary(rel, text)
        || (rel.starts_with("validator/src/cli/")
            && text.contains("PathBuf")
            && (text.contains("pub(crate) struct ")
                || text.contains("pub(crate) enum ")
                || text.contains("pub(crate) fn parse(raw: &[String])")
                || (text.contains("args: &[String]")
                    && text.contains("Result<PathBuf, String>")
                    && text.contains("is_absolute()"))))
}

fn typed_argument_parser_boundary(rel: &str, text: &str) -> bool {
    (rel == "validator/src/argument_parser.rs" || rel.starts_with("validator/src/argument_parser/"))
        && (text.contains("CliRoot")
            || text.contains("CliArtifactPath")
            || text.contains("CliText")
            || text.contains("typed_path("))
}

fn impacted_rust_tests_parser_boundary(rel: &str, text: &str) -> bool {
    rel == "validator/src/cli/live_loop/rust_tests/args.rs"
        && text.contains("ImpactedRustTestsCommand")
        && text.contains("pub(super) fn parse(raw: &[String])")
        && text.contains("opt_paths(args, \"--changed\")?")
        && text.contains("unknown impacted Rust test argument")
}

fn typed_execution_projection_boundary(rel: &str, text: &str) -> bool {
    let command_projection_path = rel == "validator/src/command/mod.rs";
    let has_marker_constant = text.contains("EXECUTION_PROJECTION_ROLE");
    let has_product_role = text.contains("execution_projection_from_typed_cli_authority");
    command_projection_path && has_marker_constant && has_product_role
}

pub(super) fn typed_law_check_boundary_text(text: &str) -> bool {
    (text.contains("-> Vec<String>")
        || text.contains("-> Vec<(String, String)>")
        || text.contains("Vec<SkillLinkFailure>")
        || text.contains("BTreeMap<String, Vec<String>>")
        || text.contains("failures: &mut"))
        && (text.contains("out.extend(")
            || text.contains("&mut out")
            || text.contains(".push(")
            || text.contains("Some(format!(")
            || text.contains("format!(\""))
}

pub(super) fn typed_path_boundary_text(rel: &str, text: &str) -> bool {
    matches!(
        rel,
        "validator/src/cli/observe/command_roundtrip/process.rs" | "validator/src/skill_links.rs"
    ) && path_boundary_product_role(text)
}

fn path_boundary_product_role(text: &str) -> bool {
    let receipt_path_validator = text.contains("validate_receipt_path")
        && text.contains("expected_receipt_path")
        && text.contains("normalize_relative_path");
    let command_process_adapter = text.contains("run_production_command")
        && text.contains("CommandOutput")
        && text.contains("current_exe_with");
    let skill_reference_parser = text.contains("SkillLinkFailure")
        && text.contains("normalized_ref")
        && text.contains("resolves_inside");
    receipt_path_validator || command_process_adapter || skill_reference_parser
}

#[cfg(test)]
mod tests {
    #[test]
    fn execution_projection_boundary_requires_path_marker_and_product_role() {
        let valid = "pub(crate) const EXECUTION_PROJECTION_ROLE: &str = \"execution_projection_from_typed_cli_authority\";";
        assert!(super::typed_execution_projection_boundary(
            "validator/src/command/mod.rs",
            valid
        ));
        assert!(!super::typed_execution_projection_boundary(
            "validator/src/command/mod.rs",
            "pub(crate) const EXECUTION_PROJECTION_ROLE: &str = \"missing\";"
        ));
        assert!(!super::typed_execution_projection_boundary(
            "validator/src/not_command/mod.rs",
            valid
        ));
    }
}
