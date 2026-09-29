use super::{imports, source};
use crate::syntax::{Report, TypeDeclaration};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Records<'a> {
    index: &'a source::Index,
    declarations: BTreeMap<String, Vec<&'a TypeDeclaration>>,
}

impl<'a> Records<'a> {
    pub(super) fn new(index: &'a source::Index, reports: &'a BTreeMap<String, Report>) -> Self {
        let mut declarations: BTreeMap<String, Vec<&TypeDeclaration>> = BTreeMap::new();
        for (path, report) in reports {
            let Some(scope) = source::scope(path) else {
                continue;
            };
            for ty in &report.types {
                declarations
                    .entry(format!("{scope}::{}", ty.name))
                    .or_default()
                    .push(ty);
            }
        }
        Self {
            index,
            declarations,
        }
    }

    pub(super) fn closed(&self, ty: &syn::Type, owner: &str) -> bool {
        self.type_closed(ty, owner, true, &mut BTreeSet::new())
    }

    fn type_closed(
        &self,
        ty: &syn::Type,
        owner: &str,
        root: bool,
        visiting: &mut BTreeSet<String>,
    ) -> bool {
        match ty {
            syn::Type::Path(path) if path.qself.is_none() => {
                let Some(last) = path.path.segments.last() else {
                    return false;
                };
                let name = last.ident.to_string();
                if root
                    && matches!(
                        name.as_str(),
                        "Value" | "String" | "Path" | "PathBuf" | "Map" | "HashMap" | "BTreeMap"
                    )
                {
                    return false;
                }
                if root
                    && matches!(name.as_str(), "Vec" | "Option")
                    && !matches!(&last.arguments, syn::PathArguments::AngleBracketed(args) if args.args.len() == 1)
                {
                    return false;
                }
                for segment in &path.path.segments {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && !args.args.iter().all(|a| match a {
                            syn::GenericArgument::Type(t) => {
                                self.type_closed(t, owner, root, visiting)
                            }
                            syn::GenericArgument::Lifetime(_) => !root,
                            _ => false,
                        })
                    {
                        return false;
                    }
                }
                let spelling = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let Some(full) = imports::absolute(owner, &spelling) else {
                    return false;
                };
                match self.resolve(&full, &mut BTreeSet::new()) {
                    Ok(Some(key)) => self.record_closed(&key, visiting),
                    Err(()) => false,
                    Ok(None) => {
                        (!root
                            && !matches!(
                                path.path.segments[0].ident.to_string().as_str(),
                                "crate" | "self" | "super"
                            ))
                            || matches!(name.as_str(), "Vec" | "Option")
                    }
                }
            }
            syn::Type::Tuple(tuple) if !root => tuple
                .elems
                .iter()
                .all(|t| self.type_closed(t, owner, false, visiting)),
            syn::Type::Reference(reference) if !root => {
                self.type_closed(&reference.elem, owner, false, visiting)
            }
            syn::Type::Array(array) if !root => {
                self.type_closed(&array.elem, owner, false, visiting)
            }
            syn::Type::Slice(slice) if !root => {
                self.type_closed(&slice.elem, owner, false, visiting)
            }
            _ => false,
        }
    }

    fn record_closed(&self, key: &str, visiting: &mut BTreeSet<String>) -> bool {
        let Some(declarations) = self.declarations.get(key) else {
            return false;
        };
        if declarations.len() != 1 || !declarations[0].closed {
            return false;
        }
        if !visiting.insert(key.into()) {
            return true;
        }
        let owner = key.rsplit_once("::").unwrap().0;
        let closed = declarations[0].fields.iter().all(|(_, field)| {
            syn::parse_str::<syn::Type>(field)
                .is_ok_and(|ty| self.type_closed(&ty, owner, false, visiting))
        });
        visiting.remove(key);
        closed
    }

    fn resolve(&self, full: &str, seen: &mut BTreeSet<String>) -> Result<Option<String>, ()> {
        if !seen.insert(full.into()) {
            return Ok(None);
        }
        let exact = self.declarations.get(full);
        let parts: Vec<_> = full.split("::").collect();
        for count in (2..=parts.len()).rev() {
            if let Some(targets) = self.index.imports.get(&parts[..count].join("::")) {
                if targets.len() != 1 || exact.is_some() {
                    return Err(());
                }
                let next = [targets[0].as_str()]
                    .into_iter()
                    .chain(parts[count..].iter().copied())
                    .collect::<Vec<_>>()
                    .join("::");
                return self.resolve(&next, seen);
            }
        }
        if let Some(declarations) = exact {
            return if declarations.len() == 1 {
                Ok(Some(full.into()))
            } else {
                Err(())
            };
        }
        let mut found = BTreeSet::new();
        for count in (1..parts.len()).rev() {
            let glob = format!("{}::*", parts[..count].join("::"));
            for target in self.index.imports.get(&glob).into_iter().flatten() {
                let mut branch = seen.clone();
                if !branch.insert(format!("glob:{glob}:{target}")) {
                    continue;
                }
                let next = [target.as_str()]
                    .into_iter()
                    .chain(parts[count..].iter().copied())
                    .collect::<Vec<_>>()
                    .join("::");
                if let Some(key) = self.resolve(&next, &mut branch)? {
                    found.insert(key);
                }
            }
        }
        if found.len() > 1 {
            Err(())
        } else {
            Ok(found.into_iter().next())
        }
    }
}
