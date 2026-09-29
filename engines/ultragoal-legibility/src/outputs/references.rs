use crate::{
    inventory::{Inventory, safe_path},
    model::SymbolRef,
    syntax::Report,
};
use std::collections::BTreeMap;
use syn::visit::Visit;

pub fn function<'a>(
    reference: &SymbolRef,
    reports: &'a BTreeMap<String, Report>,
) -> Option<&'a crate::syntax::Function> {
    if !safe_path(&reference.path) || reference.symbol.is_empty() || reference.symbol.contains('*')
    {
        return None;
    }
    let matches: Vec<_> = reports
        .get(&reference.path)?
        .functions
        .iter()
        .filter(|f| f.name == reference.symbol)
        .collect();
    (matches.len() == 1 || (!matches.is_empty() && crate::conditional::exclusive(&matches)))
        .then(|| matches[0])
}

pub fn is_test(reference: &SymbolRef, inventory: &Inventory) -> bool {
    if !safe_path(&reference.path) {
        return false;
    }
    let Some(bytes) = inventory.files.get(&reference.path) else {
        return false;
    };
    let Ok(source) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    let mut finder = Tests {
        scope: vec![],
        names: vec![],
    };
    finder.visit_file(&file);
    finder
        .names
        .iter()
        .filter(|name| **name == reference.symbol)
        .count()
        == 1
}

struct Tests {
    scope: Vec<String>,
    names: Vec<String>,
}

pub fn contains_declaration(
    target_path: &str,
    target: &str,
    producer_path: &str,
    names: &[String],
    reports: &BTreeMap<String, Report>,
) -> bool {
    let mut pending: Vec<_> = names
        .iter()
        .map(|n| (producer_path.to_string(), n.clone()))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    while let Some((preferred, name)) = pending.pop() {
        let name = name.rsplit("::").next().unwrap_or(&name).to_string();
        if !seen.insert((preferred.clone(), name.clone())) {
            continue;
        }
        let candidates: Vec<_> = reports
            .iter()
            .flat_map(|(p, r)| {
                r.types
                    .iter()
                    .filter(|t| t.name == name)
                    .map(move |t| (p, t))
            })
            .collect();
        let local: Vec<_> = candidates
            .iter()
            .filter(|(p, _)| **p == preferred)
            .collect();
        let selected = if local.len() == 1 {
            Some(*local[0])
        } else if candidates.len() == 1 {
            Some(candidates[0])
        } else {
            None
        };
        let Some((path, declaration)) = selected else {
            continue;
        };
        if path == target_path && declaration.name == target {
            return true;
        }
        for (_, field) in &declaration.fields {
            if let Ok(ty) = syn::parse_str::<syn::Type>(field) {
                let mut visitor = FieldNames(vec![]);
                visitor.visit_type(&ty);
                pending.extend(visitor.0.into_iter().map(|name| (path.clone(), name)));
            }
        }
    }
    false
}

pub fn produces(
    target_path: &str,
    target: &str,
    path: &str,
    function: &crate::syntax::Function,
    reports: &BTreeMap<String, Report>,
) -> bool {
    let mut names = function.output_identifiers.clone();
    names.extend(function.constructions.clone());
    if let Some(owner) = &function.self_type {
        for name in &mut names {
            if name == "Self" {
                *name = owner.clone();
            }
        }
    }
    for construction in &function.constructions {
        if let Some((parent, variant)) = construction.rsplit_once("::") {
            let type_name = parent.rsplit("::").next().unwrap();
            if reports
                .values()
                .flat_map(|r| &r.types)
                .any(|t| t.name == type_name && t.variants.iter().any(|v| v == variant))
            {
                names.push(parent.into());
            }
        }
    }
    contains_declaration(target_path, target, path, &names, reports)
}

struct FieldNames(Vec<String>);
impl<'ast> Visit<'ast> for FieldNames {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if let Some(last) = path.path.segments.last() {
            self.0.push(last.ident.to_string());
        }
        syn::visit::visit_type_path(self, path);
    }
}
impl<'ast> Visit<'ast> for Tests {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.scope.push(item.ident.to_string());
        syn::visit::visit_item_mod(self, item);
        self.scope.pop();
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if item.attrs.iter().any(|a| a.path().is_ident("test")) {
            self.names.push(
                self.scope
                    .iter()
                    .cloned()
                    .chain([item.sig.ident.to_string()])
                    .collect::<Vec<_>>()
                    .join("::"),
            );
        }
        syn::visit::visit_item_fn(self, item);
    }
}
