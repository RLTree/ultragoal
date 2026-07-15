use std::collections::{BTreeMap, BTreeSet};

use syn::visit::Visit;

pub(super) fn validate_authority(file: &syn::File) -> Result<(), &'static str> {
    if file.items.iter().any(|item| match item {
        syn::Item::Use(_) => false,
        syn::Item::Struct(item) => item.ident != "AttemptReservation",
        syn::Item::Fn(item) => item.sig.ident != "reserve_grant",
        syn::Item::Impl(item) => !type_is(&item.self_ty, "AttemptReservation"),
        _ => true,
    }) {
        return Err("authority-top-level-items");
    }
    let attempt = file.items.iter().find_map(|item| match item {
        syn::Item::Struct(item) if item.ident == "AttemptReservation" => Some(item),
        _ => None,
    });
    let Some(attempt) = attempt else {
        return Err("authority-owner-missing");
    };
    if !attempt.attrs.is_empty() || visibility(&attempt.vis) != "super::super" {
        return Err("authority-owner-shape");
    }
    let fields = attempt
        .fields
        .iter()
        .map(|field| {
            if !matches!(field.vis, syn::Visibility::Inherited) {
                return Err("authority-field-visible");
            }
            Ok(field.ident.as_ref().expect("named owner field").to_string())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if fields != strings(["binding", "durable", "settled", "staged", "started"]) {
        return Err("authority-fields");
    }
    let functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    if functions.len() != 1
        || !functions[0].attrs.is_empty()
        || visibility(&functions[0].vis) != "super::super"
    {
        return Err("authority-constructor-entry");
    }
    validate_constructor(functions[0])
}

pub(super) fn validate_staged(file: &syn::File) -> Result<(), &'static str> {
    if file.items.iter().any(|item| match item {
        syn::Item::Use(_) => false,
        syn::Item::Struct(item) => item.ident != "StagedCustody",
        syn::Item::Impl(item) => !type_is(&item.self_ty, "StagedCustody"),
        _ => true,
    }) {
        return Err("staged-top-level-items");
    }
    let staged = file.items.iter().find_map(|item| match item {
        syn::Item::Struct(item) if item.ident == "StagedCustody" => Some(item),
        _ => None,
    });
    let Some(staged) = staged else {
        return Err("staged-owner-missing");
    };
    if !staged.attrs.is_empty() || visibility(&staged.vis) != "super" {
        return Err("staged-owner-shape");
    }
    let syn::Fields::Unnamed(fields) = &staged.fields else {
        return Err("staged-storage-shape");
    };
    if fields.unnamed.len() != 1 || !matches!(fields.unnamed[0].vis, syn::Visibility::Inherited) {
        return Err("staged-storage-shape");
    }
    Ok(())
}

fn validate_constructor(function: &syn::ItemFn) -> Result<(), &'static str> {
    let mut constructors = Vec::new();
    Find(&mut constructors).visit_block(&function.block);
    let [constructor] = constructors.as_slice() else {
        return Err("authority-constructor-count");
    };
    if constructor.rest.is_some() {
        return Err("authority-constructor-rest");
    }
    let fields = constructor
        .fields
        .iter()
        .map(|field| (member_name(&field.member), initializer(&field.expr)))
        .collect::<BTreeMap<_, _>>();
    let expected = BTreeMap::from([
        ("binding".to_owned(), "other"),
        ("durable".to_owned(), "other"),
        ("settled".to_owned(), "false-cell"),
        ("staged".to_owned(), "empty-staged"),
        ("started".to_owned(), "false-cell"),
    ]);
    (fields == expected)
        .then_some(())
        .ok_or("authority-constructor-fields")
}

struct Find<'a>(&'a mut Vec<syn::ExprStruct>);

impl Visit<'_> for Find<'_> {
    fn visit_expr_struct(&mut self, node: &syn::ExprStruct) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|part| part.ident == "AttemptReservation")
        {
            self.0.push(node.clone());
        }
        syn::visit::visit_expr_struct(self, node);
    }
}

fn type_is(value: &syn::Type, expected: &str) -> bool {
    matches!(value, syn::Type::Path(path) if path.path.is_ident(expected))
}

fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(name) => name.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
    }
}

fn initializer(expr: &syn::Expr) -> &'static str {
    let syn::Expr::Call(call) = expr else {
        return "other";
    };
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return "other";
    };
    let names = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect::<Vec<_>>();
    if names.ends_with(&["Cell".to_owned(), "new".to_owned()])
        && matches!(call.args.first(), Some(syn::Expr::Lit(lit)) if matches!(&lit.lit, syn::Lit::Bool(value) if !value.value))
    {
        "false-cell"
    } else if names.ends_with(&["StagedCustody".to_owned(), "new".to_owned()])
        && call.args.is_empty()
    {
        "empty-staged"
    } else {
        "other"
    }
}

fn visibility(value: &syn::Visibility) -> String {
    match value {
        syn::Visibility::Inherited => "private".to_owned(),
        syn::Visibility::Restricted(item) => item
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        syn::Visibility::Public(_) => "public".to_owned(),
    }
}

fn strings<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.into_iter().map(str::to_owned).collect()
}
