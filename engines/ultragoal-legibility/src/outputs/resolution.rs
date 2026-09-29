use super::graph::Index;
use crate::syntax;
pub(super) fn joined(base: &str, tail: &str) -> String {
    if base.is_empty() {
        tail.to_string()
    } else {
        format!("{base}::{tail}")
    }
}

pub(super) fn local_namespace(
    call: &str,
    root: &str,
    module: &str,
    report: &syntax::Report,
    index: &Index,
) -> bool {
    let head = call.split("::").next().unwrap_or("");
    if report
        .modules
        .iter()
        .any(|name| name.split("::").next() == Some(head))
    {
        return true;
    }
    let prefix = format!("{}::", joined(module, head));
    index
        .keys()
        .any(|(owner, symbol)| owner == root && symbol.starts_with(&prefix))
}

pub(super) fn location(path: &str) -> Option<(String, String)> {
    let (root, relative) = if let Some(relative) = path.strip_prefix("src/") {
        (String::new(), relative)
    } else if let Some((root, relative)) = path.rsplit_once("/src/") {
        (root.to_string(), relative)
    } else {
        return None;
    };
    let mut components: Vec<_> = relative.strip_suffix(".rs")?.split('/').collect();
    if components.last() == Some(&"mod")
        || components.len() == 1 && matches!(components.last(), Some(&"lib" | &"main"))
    {
        components.pop();
    }
    Some((root, components.join("::")))
}

pub(super) fn resolve(
    call: &str,
    symbol: &str,
    file_module: &str,
    report: &syntax::Report,
) -> Vec<String> {
    if let Some(tail) = call.strip_prefix("crate::") {
        return vec![tail.to_string()];
    }
    let inline = report
        .modules
        .iter()
        .filter(|module| symbol.starts_with(&format!("{module}::")))
        .max_by_key(|module| module.len())
        .map_or("", String::as_str);
    let module = if inline.is_empty() {
        file_module.to_string()
    } else {
        joined(file_module, inline)
    };
    if let Some(tail) = call.strip_prefix("self::") {
        return vec![joined(&module, tail)];
    }
    if call.starts_with("super::") {
        let mut parents: Vec<_> = module.split("::").filter(|part| !part.is_empty()).collect();
        let mut tail = call;
        while let Some(rest) = tail.strip_prefix("super::") {
            if parents.pop().is_none() {
                return Vec::new();
            }
            tail = rest;
        }
        return vec![joined(&parents.join("::"), tail)];
    }
    if call.starts_with("::") {
        return Vec::new();
    }
    if !call.contains("::") {
        let mut scopes: Vec<_> = symbol.split("::").collect();
        let mut candidates = Vec::new();
        loop {
            candidates.push(joined(file_module, &joined(&scopes.join("::"), call)));
            if scopes.pop().is_none() {
                break;
            }
        }
        return candidates;
    }
    if let Some(tail) = call.strip_prefix("Self::")
        && let Some((owner, _)) = symbol.rsplit_once("::")
    {
        return vec![joined(file_module, &joined(owner, tail))];
    }
    vec![joined(&module, call)]
}
