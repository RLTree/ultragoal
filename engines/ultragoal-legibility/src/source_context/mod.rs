pub(crate) mod bindings;
mod exports;
pub(crate) mod imports;
pub use exports::aliases;
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod tests;
use crate::syntax;
use std::collections::BTreeMap;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Context {
    pub anyhow: Vec<String>,
    pub aliases: Vec<(String, String)>,
}

pub fn contexts(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Context> {
    let sources: BTreeMap<_, _> = files
        .iter()
        .filter(|(p, _)| p.ends_with(".rs"))
        .filter_map(|(p, b)| std::str::from_utf8(b).ok().map(|s| (p.clone(), s)))
        .collect();
    let mut modules: BTreeMap<_, Vec<String>> = BTreeMap::new();
    for path in sources.keys() {
        if let Some(key) = imports::module(path) {
            modules.entry(key).or_default().push(path.clone());
        }
    }
    let mut contexts: BTreeMap<String, Context> = BTreeMap::new();
    let mut exports: BTreeMap<String, Context> = BTreeMap::new();
    for _ in 0..=sources.len() {
        let mut next = BTreeMap::new();
        let mut next_exports = BTreeMap::new();
        for (path, source) in &sources {
            let mut context = Context::default();
            for import in imports::collect(source) {
                let candidates = imports::target(path, &import, &modules);
                let candidates: Vec<_> = candidates.iter().filter_map(|p| exports.get(p)).collect();
                let Some(first) = candidates.first() else {
                    continue;
                };
                if candidates.iter().any(|candidate| candidate != first) {
                    continue;
                }
                if let Some(local) = &import.local {
                    let original = import.source.last().unwrap();
                    let exact = import.source.join("::");
                    if first.anyhow.contains(original) {
                        context.anyhow.extend([exact, local.clone()]);
                    }
                    for (name, rhs) in &first.aliases {
                        if name == original {
                            context.aliases.extend([
                                (import.source.join("::"), rhs.clone()),
                                (local.clone(), rhs.clone()),
                            ]);
                        }
                    }
                } else {
                    context.anyhow.extend(first.anyhow.clone());
                    context.aliases.extend(first.aliases.clone());
                }
            }
            context.anyhow.sort();
            context.anyhow.dedup();
            context.aliases.sort();
            context.aliases.dedup();
            let exported = Context {
                anyhow: syntax::module_anyhow_aliases(source, &context.anyhow).unwrap_or_default(),
                aliases: syntax::module_type_aliases(source, &context.anyhow, &context.aliases)
                    .unwrap_or_default(),
            };
            next.insert(path.clone(), context);
            next_exports.insert(path.clone(), exported);
        }
        let stable = exports == next_exports && contexts == next;
        exports = next_exports;
        contexts = next;
        if stable {
            break;
        }
    }
    contexts
}
