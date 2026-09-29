use super::source::{Index, Trait};
use std::collections::BTreeMap;

pub(super) fn absolute(current: &str, path: &str) -> Option<String> {
    let mut base: Vec<_> = current.split("::").collect();
    let parts: Vec<_> = path.split("::").collect();
    let mut rest = parts.as_slice();
    if rest.first() == Some(&"crate") {
        base.truncate(1);
        rest = &rest[1..];
    } else if rest.first() == Some(&"self") {
        rest = &rest[1..];
    }
    while rest.first() == Some(&"super") {
        if base.len() < 2 {
            return None;
        }
        base.pop();
        rest = &rest[1..];
    }
    Some(
        base.into_iter()
            .chain(rest.iter().copied())
            .collect::<Vec<_>>()
            .join("::"),
    )
}

pub(super) fn collect(
    tree: &syn::UseTree,
    scope: &str,
    prefix: &[String],
    imports: &mut BTreeMap<String, Vec<String>>,
) {
    let mut path = prefix.to_vec();
    let name = match tree {
        syn::UseTree::Path(node) => {
            path.push(node.ident.to_string());
            collect(&node.tree, scope, &path, imports);
            return;
        }
        syn::UseTree::Group(node) => {
            for tree in &node.items {
                collect(tree, scope, &path, imports);
            }
            return;
        }
        syn::UseTree::Name(node) => {
            if node.ident == "self" {
                let Some(name) = path.last() else {
                    return;
                };
                name.clone()
            } else {
                path.push(node.ident.to_string());
                node.ident.to_string()
            }
        }
        syn::UseTree::Rename(node) => {
            if node.ident != "self" {
                path.push(node.ident.to_string());
            }
            node.rename.to_string()
        }
        syn::UseTree::Glob(_) => "*".into(),
    };
    if let Some(target) = absolute(scope, &path.join("::")) {
        imports
            .entry(format!("{scope}::{name}"))
            .or_default()
            .push(target);
    }
}

pub(super) fn resolve<'a>(
    index: &'a Index,
    full: &str,
    seen: &mut Vec<String>,
) -> Option<&'a Trait> {
    if seen.iter().any(|s| s == full) {
        return None;
    }
    seen.push(full.into());
    let exact: Vec<_> = index.traits.iter().filter(|t| t.key == full).collect();
    let parts: Vec<_> = full.split("::").collect();
    for count in (2..=parts.len()).rev() {
        let prefix = parts[..count].join("::");
        if let Some(targets) = index.imports.get(&prefix) {
            if targets.len() != 1 || !exact.is_empty() {
                return None;
            }
            let suffix = parts[count..].join("::");
            let next = if suffix.is_empty() {
                targets[0].clone()
            } else {
                format!("{}::{suffix}", targets[0])
            };
            return resolve(index, &next, seen);
        }
    }
    (exact.len() == 1).then(|| exact[0])
}
