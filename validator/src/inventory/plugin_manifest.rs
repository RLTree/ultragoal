use super::fs::read_bounded;
use crate::context::ReadSession;
use crate::plugin_manifest::{self, PluginManifest, PluginMcpServers, SemanticIssue};
use serde_json::Value;
use std::path::Path;

pub(super) const MAX_LIST_ITEMS: usize = plugin_manifest::ITEM_LIMIT;

pub(super) type Problem = (&'static str, &'static str);

pub(super) struct Inspection {
    pub problems: Vec<Problem>,
    pub hook_paths: Vec<String>,
    pub hooks_declared: bool,
}

pub(super) fn bounded_text(value: &str) -> bool {
    plugin_manifest::bounded_text(value)
}

fn validate(
    reads: &ReadSession,
    root: &Path,
    manifest: &PluginManifest,
    problems: &mut Vec<Problem>,
) {
    for issue in plugin_manifest::semantic_issues(manifest) {
        problems.push(problem(issue));
    }
    if manifest
        .skills
        .as_deref()
        .is_some_and(|value| !super::plugin_manifest_path::matches(value, "skills"))
        || (manifest.name == "harness-ultragoal" && manifest.skills.is_none())
        || manifest.apps.as_deref().is_some_and(|value| {
            !super::plugin_manifest_path::matches(value, ".app.json")
                || !super::plugin_manifest_path::companion_exists(reads, root, value)
        })
        || manifest
            .mcp_servers
            .as_ref()
            .is_some_and(|value| match value {
                PluginMcpServers::Path(path) => {
                    !super::plugin_manifest_path::matches(path, ".mcp.json")
                        || !super::plugin_manifest_path::companion_exists(reads, root, path)
                }
                PluginMcpServers::Inline(_) => false,
            })
    {
        problems.push((
            "invalid_plugin_manifest_path",
            "source plugin manifest component path is invalid",
        ));
    }
    if let Some(interface) = &manifest.interface {
        super::plugin_manifest_interface::validate_assets(reads, root, interface, problems);
    }
}

fn problem(issue: SemanticIssue) -> Problem {
    match issue {
        SemanticIssue::Metadata => (
            "invalid_source_plugin_manifest",
            "source plugin manifest has invalid metadata",
        ),
        SemanticIssue::AuthorEmail => (
            "invalid_source_plugin_manifest",
            "source plugin manifest has invalid author email",
        ),
        SemanticIssue::Url => (
            "invalid_plugin_manifest_url",
            "source plugin manifest URL is not absolute supported HTTPS",
        ),
        SemanticIssue::InterfaceMetadata => (
            "invalid_source_plugin_manifest",
            "source plugin manifest has invalid interface metadata",
        ),
        SemanticIssue::DefaultPrompt => (
            "invalid_plugin_default_prompt",
            "source plugin manifest default prompts are invalid",
        ),
        SemanticIssue::UnsupportedDefaultPrompt => (
            "plugin_default_prompt_truncated",
            "supported host ignores unsupported plugin default prompt entries",
        ),
        SemanticIssue::BrandColor => (
            "invalid_plugin_brand_color",
            "source plugin manifest brand color is invalid",
        ),
        SemanticIssue::McpServer => (
            "invalid_plugin_mcp_server",
            "source plugin manifest MCP server definition is invalid",
        ),
    }
}

fn inspect_legacy_hooks(
    reads: &ReadSession,
    root: &Path,
    value: &Value,
    problems: &mut Vec<Problem>,
) -> Vec<String> {
    let inspection = super::plugin_manifest_hooks::inspect(reads, root, value);
    if !inspection.valid {
        problems.push((
            "invalid_plugin_hooks",
            "source plugin manifest hooks are malformed or unsafe",
        ));
    } else if inspection.inactive && inspection.paths.is_empty() {
        problems.push((
            "inactive_plugin_hook_configuration",
            "source plugin manifest contains hook configuration the current host parses but skips",
        ));
    }
    if inspection.valid && inspection.trust_required && inspection.paths.is_empty() {
        problems.push((
            "plugin_hook_trust_required",
            "source plugin command hooks require current host trust before they can run",
        ));
    }
    inspection.paths
}

pub(super) fn inspect(reads: &ReadSession, root: &Path, path: &Path) -> Inspection {
    let mut problems = Vec::new();
    let bytes = read_bounded(reads, path, plugin_manifest::MANIFEST_LIMIT as u64).ok();
    if bytes
        .as_deref()
        .is_some_and(|bytes| bytes.windows(b"[TODO:".len()).any(|part| part == b"[TODO:"))
    {
        problems.push((
            "plugin_manifest_placeholder",
            "source plugin manifest contains an unresolved placeholder",
        ));
    }
    let mut hook_paths = Vec::new();
    let mut hooks_declared = false;
    let parsed = bytes.as_deref().and_then(|bytes| {
        plugin_manifest::parse_value(bytes, plugin_manifest::MANIFEST_LIMIT).ok()
    });
    let manifest = match parsed {
        Some(Value::Object(mut object)) => {
            if let Some(hooks) = object.remove("hooks") {
                hooks_declared = true;
                problems.push((
                    "unsupported_plugin_manifest_field",
                    "source plugin manifest hooks are legacy context, not supported manifest authority",
                ));
                hook_paths = inspect_legacy_hooks(reads, root, &hooks, &mut problems);
            }
            plugin_manifest::decode_value(Value::Object(object)).ok()
        }
        _ => None,
    };
    match manifest {
        Some(manifest) => validate(reads, root, &manifest, &mut problems),
        None => problems.push((
            "invalid_source_plugin_manifest",
            "source plugin manifest is not valid bounded JSON",
        )),
    }
    problems.sort();
    problems.dedup();
    hook_paths.sort();
    hook_paths.dedup();
    Inspection {
        problems,
        hook_paths,
        hooks_declared,
    }
}
