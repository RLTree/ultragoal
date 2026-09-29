use super::imports;
use crate::syntax::Report;
use std::collections::BTreeMap;
pub type Site = (String, String);
pub type Aliases = Vec<((String, String), Vec<Site>)>;

pub fn aliases(files: &BTreeMap<String, Vec<u8>>, reports: &BTreeMap<String, Report>) -> Aliases {
    let mut modules: BTreeMap<_, Vec<String>> = BTreeMap::new();
    let mut exports: BTreeMap<String, BTreeMap<String, Vec<Site>>> = BTreeMap::new();
    for (path, report) in reports {
        if let Some(key) = imports::module(path) {
            modules.entry(key).or_default().push(path.clone());
        }
        exports.insert(
            path.clone(),
            report
                .functions
                .iter()
                .map(|f| (f.name.clone(), vec![(path.clone(), f.name.clone())]))
                .collect(),
        );
    }
    let imports: BTreeMap<_, _> = files
        .iter()
        .filter_map(|(p, b)| {
            std::str::from_utf8(b)
                .ok()
                .map(|s| (p.clone(), imports::collect(s)))
        })
        .collect();
    for _ in 0..=reports.len() {
        let mut next = exports.clone();
        for path in reports.keys() {
            for import in imports.get(path).into_iter().flatten() {
                for target in imports::target(path, import, &modules) {
                    for (name, sites) in exports.get(&target).into_iter().flatten() {
                        let alias = if let Some(local) = &import.local {
                            let original = import.source.last().unwrap();
                            if name == original {
                                local.clone()
                            } else if let Some(suffix) = name.strip_prefix(&format!("{original}::"))
                            {
                                format!("{local}::{suffix}")
                            } else {
                                continue;
                            }
                        } else {
                            name.clone()
                        };
                        let current = next
                            .entry(path.clone())
                            .or_default()
                            .entry(alias)
                            .or_default();
                        current.extend(sites.clone());
                        current.sort();
                        current.dedup();
                    }
                }
            }
        }
        if next == exports {
            break;
        }
        exports = next;
    }
    let mut aliases = vec![];
    for (path, symbols) in exports {
        let Some((package, module)) = imports::module(&path) else {
            continue;
        };
        let root = if package == "project" {
            String::new()
        } else {
            package
        };
        for (name, sites) in symbols {
            let qualified = module
                .iter()
                .cloned()
                .chain([name])
                .collect::<Vec<_>>()
                .join("::");
            aliases.push(((root.clone(), qualified), sites));
        }
    }
    aliases
}
