use super::super::{
    ITEM_LIMIT, PluginMcpServer, PluginMcpServers, bounded_text, environment_map, header_map,
    https, kebab, unique_list,
};
use super::SemanticIssue;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(value: &PluginMcpServers, issues: &mut BTreeSet<SemanticIssue>) {
    let valid = match value {
        PluginMcpServers::Path(path) => bounded_text(path),
        PluginMcpServers::Inline(servers) => inline_valid(servers),
    };
    if !valid {
        issues.insert(SemanticIssue::McpServer);
    }
}

fn inline_valid(servers: &BTreeMap<String, PluginMcpServer>) -> bool {
    !servers.is_empty()
        && servers.len() <= ITEM_LIMIT
        && servers
            .iter()
            .all(|(name, server)| kebab(name) && server_valid(server))
}

fn server_valid(server: &PluginMcpServer) -> bool {
    let valid_shared_settings = unique_list(&server.args, ITEM_LIMIT)
        && environment_map(&server.env)
        && header_map(&server.headers);
    let stdio = server.kind == "stdio"
        && server.command.as_deref().is_some_and(bounded_text)
        && server.url.is_none()
        && server.headers.is_empty();
    let http = server.kind == "http"
        && server.url.as_deref().is_some_and(https)
        && server.command.is_none()
        && server.args.is_empty()
        && server.env.is_empty();
    valid_shared_settings && (stdio || http)
}
