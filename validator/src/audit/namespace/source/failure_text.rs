pub(super) fn remediating_failure(
    code: &str,
    directory: &str,
    prefix: &str,
    examples: &[String],
    repair: &str,
    exception_allowed: bool,
) -> String {
    let sample = examples
        .iter()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{code}:directory={directory};prefix={prefix};count={};examples={sample};why=prefix_or_history_name_is_standing_in_for_semantic_directory;repair={repair};claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed={exception_allowed}",
        examples.len()
    )
}
