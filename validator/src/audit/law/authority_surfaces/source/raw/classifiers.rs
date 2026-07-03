pub(super) fn typed_cli_command_boundary_text(rel: &str, text: &str) -> bool {
    rel == "validator/src/command/mod.rs"
        || (rel.starts_with("validator/src/cli/")
            && text.contains("PathBuf")
            && (text.contains("pub(crate) struct ")
                || text.contains("pub(crate) enum ")
                || text.contains("pub(crate) fn parse(raw: &[String])")))
}

pub(super) fn typed_law_check_boundary_text(text: &str) -> bool {
    (text.contains("-> Vec<String>")
        || text.contains("BTreeMap<String, Vec<String>>")
        || text.contains("failures: &mut"))
        && (text.contains("out.extend(")
            || text.contains("&mut out")
            || text.contains(".push(")
            || text.contains("Some(format!(")
            || text.contains("format!(\""))
}
