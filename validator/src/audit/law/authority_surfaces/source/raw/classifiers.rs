pub(super) fn typed_cli_command_boundary_text(rel: &str, text: &str) -> bool {
    rel == "validator/src/argument_parser.rs"
        || rel == "validator/src/command/mod.rs"
        || (rel.starts_with("validator/src/cli/")
            && text.contains("PathBuf")
            && (text.contains("pub(crate) struct ")
                || text.contains("pub(crate) enum ")
                || text.contains("pub(crate) fn parse(raw: &[String])")
                || (text.contains("args: &[String]")
                    && text.contains("Result<PathBuf, String>")
                    && text.contains("is_absolute()"))))
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
        "validator/src/cli/control/plane/path.rs"
            | "validator/src/cli/observe/command_roundtrip/process.rs"
            | "validator/src/audit/receipt/generated.rs"
            | "validator/src/skill_links.rs"
    ) && path_boundary_product_role(text)
}

fn path_boundary_product_role(text: &str) -> bool {
    let receipt_path_validator = text.contains("validate_receipt_path")
        && text.contains("expected_receipt_path")
        && text.contains("normalize_relative_path");
    let command_process_adapter = text.contains("run_production_command")
        && text.contains("CommandOutput")
        && text.contains("current_exe_with");
    let generated_artifact_normalizer = text.contains("ReceiptInput")
        && text.contains("target_artifacts")
        && text.contains("super::rel_path");
    let skill_reference_parser = text.contains("SkillLinkFailure")
        && text.contains("normalized_ref")
        && text.contains("resolves_inside");
    receipt_path_validator
        || command_process_adapter
        || generated_artifact_normalizer
        || skill_reference_parser
}
