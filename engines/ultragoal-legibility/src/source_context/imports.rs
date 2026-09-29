use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Import {
    pub source: Vec<String>,
    pub local: Option<String>,
}

fn flatten(tree: &syn::UseTree, prefix: &[String], imports: &mut Vec<Import>) {
    match tree {
        syn::UseTree::Path(path) => {
            let mut prefix = prefix.to_vec();
            prefix.push(path.ident.to_string());
            flatten(&path.tree, &prefix, imports);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                flatten(item, prefix, imports);
            }
        }
        syn::UseTree::Name(name) => {
            let mut source = prefix.to_vec();
            if name.ident != "self" {
                source.push(name.ident.to_string());
            }
            imports.push(Import {
                source,
                local: Some(if name.ident == "self" {
                    prefix.last().cloned().unwrap_or_default()
                } else {
                    name.ident.to_string()
                }),
            });
        }
        syn::UseTree::Rename(rename) => {
            let mut source = prefix.to_vec();
            if rename.ident != "self" {
                source.push(rename.ident.to_string());
            }
            imports.push(Import {
                source,
                local: Some(rename.rename.to_string()),
            });
        }
        syn::UseTree::Glob(_) => imports.push(Import {
            source: prefix.to_vec(),
            local: None,
        }),
    }
}

pub fn collect(source: &str) -> Vec<Import> {
    let Ok(file) = syn::parse_file(source) else {
        return vec![];
    };
    let mut imports = vec![];
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            flatten(&item.tree, &[], &mut imports);
        }
    }
    imports
}

pub fn module(path: &str) -> Option<(String, Vec<String>)> {
    let (package, source) = if let Some((package, source)) = path.rsplit_once("/src/") {
        (package, source)
    } else {
        ("project", path.strip_prefix("src/")?)
    };
    let mut parts: Vec<_> = source
        .strip_suffix(".rs")?
        .split('/')
        .map(str::to_owned)
        .collect();
    if parts
        .last()
        .is_some_and(|p| matches!(p.as_str(), "mod" | "lib" | "main"))
    {
        parts.pop();
    }
    Some((package.into(), parts))
}

pub fn target(
    path: &str,
    import: &Import,
    modules: &BTreeMap<(String, Vec<String>), Vec<String>>,
) -> Vec<String> {
    let Some((package, current)) = module(path) else {
        return vec![];
    };
    let source = &import.source[..import
        .source
        .len()
        .saturating_sub(usize::from(import.local.is_some()))];
    let mut target = current.clone();
    let mut rest = source;
    if rest.first().is_some_and(|s| s == "crate") {
        target.clear();
        rest = &rest[1..];
    } else if rest.first().is_some_and(|s| s == "self") {
        rest = &rest[1..];
    }
    while rest.first().is_some_and(|s| s == "super") {
        if target.pop().is_none() {
            return vec![];
        }
        rest = &rest[1..];
    }
    target.extend_from_slice(rest);
    modules
        .get(&(package.clone(), target))
        .or_else(|| modules.get(&(package, source.to_vec())))
        .cloned()
        .unwrap_or_default()
}
