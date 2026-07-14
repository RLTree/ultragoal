mod cfg;
mod filter;
mod model;
mod ownership;

pub(crate) use model::{ProductionSource, ProductionSourceSet};

use super::{GovernedInventory, scope::SourceClass};
use std::collections::BTreeMap;

pub(crate) fn production_sources(inventory: &GovernedInventory) -> ProductionSourceSet {
    let governed = inventory
        .sources
        .iter()
        .filter(|source| source.class == SourceClass::RustProduction)
        .collect::<Vec<_>>();
    let mut parsed = BTreeMap::new();
    let mut failures = Vec::new();
    for source in &governed {
        let text = match std::str::from_utf8(&source.bytes) {
            Ok(value) => value,
            Err(_) => {
                failures.push(format!("production_source_non_utf8:{}", source.relative));
                continue;
            }
        };
        match syn::parse_file(text) {
            Ok(file) => {
                parsed.insert(source.relative.clone(), file);
            }
            Err(_) => failures.push(format!(
                "production_source_syntax_invalid:{}",
                source.relative
            )),
        }
    }
    let ownership = ownership::analyze(&parsed);
    failures.extend(ownership.failures);
    let mut sources = Vec::new();
    for source in governed {
        if ownership.test_only.contains(&source.relative) {
            continue;
        }
        let bytes = match std::str::from_utf8(&source.bytes) {
            Ok(text) if parsed.contains_key(&source.relative) => {
                match filter::production_text(&source.relative, text) {
                    Ok(filtered) => filtered.into_bytes(),
                    Err(failure) => {
                        failures.push(failure);
                        source.bytes.clone()
                    }
                }
            }
            _ => source.bytes.clone(),
        };
        sources.push(ProductionSource {
            relative: source.relative.clone(),
            bytes,
        });
    }
    sources.sort_by(|left, right| left.relative.cmp(&right.relative));
    failures.sort();
    failures.dedup();
    ProductionSourceSet { sources, failures }
}
