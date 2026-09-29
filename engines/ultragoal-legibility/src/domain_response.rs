use crate::{
    model::BoundaryRow,
    syntax::{Function, Report, TypeDeclaration},
};
use std::collections::BTreeMap;

fn inner(mut ty: &syn::Type) -> Option<&syn::TypePath> {
    loop {
        let syn::Type::Path(path) = ty else {
            return None;
        };
        let last = path.path.segments.last()?;
        if !matches!(last.ident.to_string().as_str(), "Option" | "Vec") {
            return Some(path);
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        if args.args.len() != 1 {
            return None;
        }
        let Some(syn::GenericArgument::Type(value)) = args.args.first() else {
            return None;
        };
        ty = value;
    }
}

fn declaration<'a>(
    name: &str,
    preferred: &str,
    reports: &'a BTreeMap<String, Report>,
) -> Option<(&'a str, &'a TypeDeclaration)> {
    let matches: Vec<_> = reports
        .iter()
        .flat_map(|(p, r)| {
            r.types
                .iter()
                .filter(move |t| t.name == name)
                .map(move |t| (p.as_str(), t))
        })
        .collect();
    let local: Vec<_> = matches.iter().filter(|(p, _)| *p == preferred).collect();
    if local.len() == 1 {
        return Some(**local.first().unwrap());
    }
    (matches.len() == 1).then(|| matches[0])
}

pub fn closed(row: &BoundaryRow, function: &Function, reports: &BTreeMap<String, Report>) -> bool {
    if row.response.is_empty() {
        return false;
    }
    let Ok(returned) = syn::parse_str::<syn::Type>(&function.return_type) else {
        return false;
    };
    let Some(path) = inner(&returned) else {
        return false;
    };
    let name = row.response.rsplit("::").next().unwrap();
    if path.path.segments.last().is_none_or(|s| s.ident != name) {
        return false;
    }
    let Some((mut owner, mut record)) = declaration(name, &row.path, reports) else {
        return false;
    };
    if let Some(fields) = &row.error_field {
        let parts: Vec<_> = fields.split('.').collect();
        for (index, field) in parts.iter().enumerate() {
            let Some((_, raw)) = record.fields.iter().find(|(n, _)| n == field) else {
                return false;
            };
            let Ok(ty) = syn::parse_str::<syn::Type>(raw) else {
                return false;
            };
            let Some(nested) = inner(&ty) else {
                return false;
            };
            let nested_name = nested.path.segments.last().unwrap().ident.to_string();
            if index + 1 == parts.len() && row.error_variant.is_none() {
                // Optional/collection error data or a boolean flag carries failure state;
                // an always-present explanatory String alone does not.
                return matches!(&ty,syn::Type::Path(p) if p.path.segments.last().is_some_and(|s|matches!(s.ident.to_string().as_str(),"Option"|"Vec"|"bool")));
            }
            let Some(found) = declaration(&nested_name, owner, reports) else {
                return false;
            };
            (owner, record) = found;
        }
    }
    row.error_variant
        .as_ref()
        .is_some_and(|v| record.variants.contains(v))
}
