use std::collections::BTreeMap;
use syn::{Item, UseTree};

pub(super) type Aliases = BTreeMap<String, Vec<String>>;

pub(super) fn collect(items: &[Item]) -> (Aliases, Vec<String>) {
    let mut aliases = Aliases::new();
    let mut paths = Vec::new();
    for item in items {
        if super::declarations::test_only(item) {
            continue;
        }
        if let Item::ExternCrate(item) = item {
            let local = item
                .rename
                .as_ref()
                .map(|(_, name)| name)
                .unwrap_or(&item.ident);
            aliases.insert(local.to_string(), vec![item.ident.to_string()]);
            continue;
        }
        let declared = match item {
            Item::Fn(i) => Some(&i.sig.ident),
            Item::Struct(i) => Some(&i.ident),
            Item::Enum(i) => Some(&i.ident),
            Item::Type(i) => Some(&i.ident),
            Item::Mod(i) => Some(&i.ident),
            Item::Const(i) => Some(&i.ident),
            Item::Static(i) => Some(&i.ident),
            Item::Trait(i) => Some(&i.ident),
            _ => None,
        };
        if let Some(name) = declared {
            aliases
                .entry(name.to_string())
                .or_default()
                .push(name.to_string());
        }
        match item {
            Item::Use(item) => tree(&item.tree, &[], &mut aliases, &mut paths),
            Item::ExternCrate(item) => {
                let root = item.ident.to_string();
                let name = item
                    .rename
                    .as_ref()
                    .map_or(root.clone(), |(_, x)| x.to_string());
                aliases.entry(name).or_default().push(root.clone());
                paths.push(root);
            }
            _ => {}
        }
    }
    (aliases, paths)
}

fn tree(node: &UseTree, prefix: &[String], aliases: &mut Aliases, paths: &mut Vec<String>) {
    let mut path = prefix.to_vec();
    match node {
        UseTree::Path(node) => {
            path.push(node.ident.to_string());
            tree(&node.tree, &path, aliases, paths);
        }
        UseTree::Name(node) => {
            let name = node.ident.to_string();
            if name != "self" {
                path.push(name.clone());
            }
            let alias = if name == "self" {
                prefix.last().cloned().unwrap_or(name)
            } else {
                name
            };
            insert(alias, path, aliases, paths);
        }
        UseTree::Rename(node) => {
            if node.ident != "self" {
                path.push(node.ident.to_string());
            }
            insert(node.rename.to_string(), path, aliases, paths);
        }
        UseTree::Group(node) => {
            for node in &node.items {
                tree(node, &path, aliases, paths);
            }
        }
        UseTree::Glob(_) => {
            aliases.entry("*".into()).or_default().push(path.join("::"));
            path.push("*".into());
            paths.push(path.join("::"));
        }
    }
}

fn insert(name: String, path: Vec<String>, aliases: &mut Aliases, paths: &mut Vec<String>) {
    let full = path.join("::");
    aliases.entry(name).or_default().push(full.clone());
    paths.push(full);
}

pub(super) fn resolve(path: &syn::Path, scopes: &[Aliases]) -> Vec<String> {
    let parts = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>();
    if path.leading_colon.is_some() {
        return vec![parts.join("::")];
    }
    let mut resolved = resolve_parts(&parts, scopes, &mut Vec::new());
    let explicit = parts
        .first()
        .is_some_and(|first| scopes.iter().any(|scope| scope.contains_key(first)));
    if !explicit
        && !matches!(
            parts.first().map(String::as_str),
            Some(
                "crate"
                    | "self"
                    | "super"
                    | "std"
                    | "core"
                    | "alloc"
                    | "anyhow"
                    | "serde_json"
                    | "serde_saphyr"
                    | "toml"
                    | "syn"
                    | "quote"
                    | "proc_macro2"
                    | "libc"
                    | "clap"
                    | "serde"
                    | "sha2"
                    | "tempfile"
            )
        )
    {
        for prefix in scopes.iter().filter_map(|scope| scope.get("*")).flatten() {
            let mut candidate = prefix.split("::").map(str::to_owned).collect::<Vec<_>>();
            candidate.extend(parts.clone());
            resolved.extend(resolve_parts(&candidate, scopes, &mut Vec::new()));
        }
    }
    resolved.sort();
    resolved.dedup();
    resolved
}

fn resolve_parts(parts: &[String], scopes: &[Aliases], seen: &mut Vec<String>) -> Vec<String> {
    let Some(first) = parts.first() else {
        return Vec::new();
    };
    if seen.contains(first) {
        return vec![parts.join("::")];
    }
    if parts.len() > 1
        && let Some(targets) = scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&parts.join("::")))
    {
        return targets.clone();
    }
    let aliases = scopes.iter().rev().find_map(|scope| scope.get(first));
    let Some(aliases) = aliases else {
        return vec![parts.join("::")];
    };
    seen.push(first.clone());
    let mut resolved = Vec::new();
    for alias in aliases {
        let mut next = alias.split("::").map(str::to_owned).collect::<Vec<_>>();
        next.extend_from_slice(&parts[1..]);
        resolved.extend(resolve_parts(&next, scopes, &mut seen.clone()));
    }
    resolved.sort();
    resolved.dedup();
    resolved
}
