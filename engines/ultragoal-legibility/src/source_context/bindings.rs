//! Resolve actual source imports before applying lexical Rust prelude fallback.
use super::imports;
use std::collections::BTreeMap;
use syn::Item;
pub type Bindings = BTreeMap<String, Vec<String>>;

fn canonical(path: &str, name: &str) -> Option<String> {
    let (_, module) = imports::module(path)?;
    Some(
        std::iter::once("crate".into())
            .chain(module)
            .chain([name.into()])
            .collect::<Vec<String>>()
            .join("::"),
    )
}

pub fn contexts(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Bindings> {
    let mut modules: BTreeMap<_, Vec<String>> = BTreeMap::new();
    let mut declared = BTreeMap::<String, Bindings>::new();
    let mut uses = BTreeMap::new();
    for (path, bytes) in files {
        let Some(key) = imports::module(path) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(file) = syn::parse_file(text) else {
            continue;
        };
        modules.entry(key).or_default().push(path.clone());
        let mut own = Bindings::new();
        for item in &file.items {
            let name = match item {
                Item::Fn(i) => Some(&i.sig.ident),
                Item::Struct(i) => Some(&i.ident),
                Item::Enum(i) => Some(&i.ident),
                Item::Type(i) => Some(&i.ident),
                Item::Mod(i) => Some(&i.ident),
                Item::Trait(i) => Some(&i.ident),
                Item::Const(i) => Some(&i.ident),
                Item::Static(i) => Some(&i.ident),
                _ => None,
            };
            if let Some(name) = name {
                own.insert(
                    name.to_string(),
                    vec![canonical(path, &name.to_string()).unwrap()],
                );
            }
        }
        declared.insert(path.clone(), own);
        uses.insert(path.clone(), imports::collect(text));
    }
    let mut exports = declared.clone();
    let mut contexts = BTreeMap::new();
    for _ in 0..=declared.len() {
        let mut next = declared.clone();
        let mut next_contexts = BTreeMap::new();
        for (path, imports_here) in &uses {
            let mut bindings = Bindings::new();
            for import in imports_here {
                let targets = imports::target(path, import, &modules);
                if let Some(local) = &import.local {
                    let original = import.source.last().unwrap();
                    let mut resolved = vec![];
                    for target in &targets {
                        resolved.extend(
                            exports
                                .get(target)
                                .and_then(|e| e.get(original))
                                .into_iter()
                                .flatten()
                                .cloned(),
                        );
                    }
                    if resolved.is_empty()
                        && import.source.first().is_some_and(|s| {
                            matches!(
                                s.as_str(),
                                "std"
                                    | "core"
                                    | "alloc"
                                    | "anyhow"
                                    | "serde"
                                    | "serde_json"
                                    | "syn"
                                    | "quote"
                                    | "libc"
                                    | "proc_macro2"
                                    | "sha2"
                                    | "clap"
                                    | "tempfile"
                                    | "serde_saphyr"
                            )
                        })
                    {
                        resolved.push(import.source.join("::"));
                    }
                    if !resolved.is_empty() {
                        bindings
                            .entry(local.clone())
                            .or_default()
                            .extend(resolved.clone());
                        bindings
                            .entry(import.source.join("::"))
                            .or_default()
                            .extend(resolved);
                    }
                } else {
                    for target in targets {
                        for (name, values) in exports.get(&target).into_iter().flatten() {
                            bindings
                                .entry(name.clone())
                                .or_default()
                                .extend(values.clone());
                            bindings
                                .entry(format!("{}::{name}", import.source.join("::")))
                                .or_default()
                                .extend(values.clone());
                        }
                    }
                }
            }
            for values in bindings.values_mut() {
                values.sort();
                values.dedup();
            }
            let exported = next.get_mut(path).unwrap();
            for (name, values) in &bindings {
                if !name.contains("::") && !declared[path].contains_key(name) {
                    exported.insert(name.clone(), values.clone());
                }
            }
            next_contexts.insert(path.clone(), bindings);
        }
        let stable = exports == next;
        exports = next;
        contexts = next_contexts;
        if stable {
            break;
        }
    }
    contexts
}

pub fn prelude() -> Bindings {
    [
        ("Ok", "std::result::Result::Ok"),
        ("Err", "std::result::Result::Err"),
        ("Some", "std::option::Option::Some"),
        ("None", "std::option::Option::None"),
        ("Vec", "std::vec::Vec"),
        ("String", "std::string::String"),
        ("Box", "std::boxed::Box"),
        ("Option", "std::option::Option"),
        ("Result", "std::result::Result"),
        ("Default", "std::default::Default"),
        ("i32", "i32"),
        ("u32", "u32"),
        ("u64", "u64"),
        ("usize", "usize"),
    ]
    .into_iter()
    .map(|(a, b)| (a.into(), vec![b.into()]))
    .collect()
}
