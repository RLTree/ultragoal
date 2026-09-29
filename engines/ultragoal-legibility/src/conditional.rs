//! Prove disjoint cfg alternatives; never select an arbitrary duplicate.
use crate::syntax::Function;
use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use syn::{Meta, Token, parse::Parser, punctuated::Punctuated};

fn children(meta: &Meta) -> Option<Vec<Meta>> {
    match meta {
        Meta::List(l) => Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(l.tokens.clone())
            .ok()
            .map(|v| v.into_iter().collect()),
        _ => None,
    }
}
fn key(meta: &Meta) -> String {
    meta.to_token_stream().to_string().replace(' ', "")
}
fn atoms(meta: &Meta, out: &mut BTreeSet<String>) {
    if let Some(items) = children(meta) {
        for item in items {
            atoms(&item, out)
        }
    } else {
        out.insert(key(meta));
    }
}
fn evaluate(meta: &Meta, values: &BTreeMap<String, bool>) -> bool {
    match meta {
        Meta::List(list) => {
            let Some(items) = children(meta) else {
                return true;
            };
            if list.path.is_ident("not") && items.len() == 1 {
                !evaluate(&items[0], values)
            } else if list.path.is_ident("all") {
                items.iter().all(|m| evaluate(m, values))
            } else if list.path.is_ident("any") {
                items.iter().any(|m| evaluate(m, values))
            } else {
                true
            }
        }
        _ => values.get(&key(meta)).copied().unwrap_or(true),
    }
}
fn possible(values: &BTreeMap<String, bool>) -> bool {
    let mut fixed = BTreeSet::new();
    for (key, value) in values {
        if !value {
            continue;
        }
        if let Some((name, _)) = key.split_once('=')
            && matches!(
                name,
                "target_os"
                    | "target_arch"
                    | "target_env"
                    | "target_vendor"
                    | "target_endian"
                    | "target_pointer_width"
            )
            && !fixed.insert(name)
        {
            return false;
        }
    }
    !(values.get("unix") == Some(&true) && values.get("windows") == Some(&true))
}
pub fn exclusive(functions: &[&Function]) -> bool {
    if functions.len() < 2 {
        return false;
    }
    let mut groups = vec![];
    let mut names = BTreeSet::new();
    for function in functions {
        if function.conditions.is_empty() {
            return false;
        }
        let parsed = function
            .conditions
            .iter()
            .map(|s| syn::parse_str::<Meta>(s))
            .collect::<Result<Vec<_>, _>>();
        let Ok(parsed) = parsed else {
            return false;
        };
        for meta in &parsed {
            atoms(meta, &mut names);
        }
        groups.push(parsed);
    }
    if names.len() > 12 {
        return false;
    }
    let names: Vec<_> = names.into_iter().collect();
    for mask in 0usize..(1 << names.len()) {
        let values = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), mask & (1 << i) != 0))
            .collect();
        if possible(&values)
            && groups
                .iter()
                .filter(|g| g.iter().all(|m| evaluate(m, &values)))
                .count()
                > 1
        {
            return false;
        }
    }
    true
}
