pub(super) fn failures(paths: &[String]) -> Vec<String> {
    paths.iter().filter_map(|path| path_failure(path)).collect()
}

fn path_failure(path: &str) -> Option<String> {
    let components = path.split('/').collect::<Vec<_>>();
    let valid = match components.as_slice() {
        ["skills", skill, "SKILL.md"] => semantic_slug(skill),
        ["skills", skill, "agents", "openai.yaml"] => semantic_slug(skill),
        ["skills", skill, role, tail @ ..]
            if matches!(*role, "assets" | "references" | "scripts") =>
        {
            semantic_slug(skill) && !tail.is_empty() && tail.iter().all(|part| semantic_leaf(part))
        }
        ["agents", leaf] => semantic_extension_leaf(leaf, "md"),
        ["custom-agents", leaf] | [".codex", "agents", leaf] => {
            semantic_extension_leaf(leaf, "toml")
        }
        [".codex-plugin", "plugin.json"] => true,
        [root, tail @ ..] if matches!(*root, "hooks" | "mcp" | "config") => {
            !tail.is_empty() && tail.iter().all(|part| semantic_leaf(part))
        }
        _ if plugin_interface_path(path) => false,
        _ => return None,
    };
    (!valid).then(|| {
        format!(
            "namespace_plugin_interface_route_invalid:{path}:repair=use_semantic_skill_agent_hook_mcp_or_config_route"
        )
    })
}

fn plugin_interface_path(path: &str) -> bool {
    [
        "skills/",
        "agents/",
        "custom-agents/",
        ".codex/agents/",
        ".codex-plugin/",
        "hooks/",
        "mcp/",
        "config/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn semantic_extension_leaf(leaf: &str, extension: &str) -> bool {
    leaf.strip_suffix(&format!(".{extension}"))
        .is_some_and(semantic_slug)
}

fn semantic_leaf(leaf: &str) -> bool {
    let stem = leaf
        .rsplit_once('.')
        .map(|(value, _)| value)
        .unwrap_or(leaf);
    semantic_slug(stem)
}

fn semantic_slug(value: &str) -> bool {
    !value.is_empty()
        && !matches!(
            value,
            "helper" | "helpers" | "utils" | "common" | "shared" | "misc"
        )
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    #[test]
    fn plugin_routes_require_semantic_product_paths() {
        assert!(super::path_failure("skills/product-fit/SKILL.md").is_none());
        assert!(super::path_failure("agents/product-reviewer.md").is_none());
        assert!(super::path_failure("skills/helpers/SKILL.md").is_some());
        assert!(super::path_failure(".codex-plugin/other.json").is_some());
    }
}
