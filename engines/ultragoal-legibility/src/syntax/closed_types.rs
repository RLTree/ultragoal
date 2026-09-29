pub(super) fn closed_error(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    path.path.segments.last().is_some_and(|last| {
        !matches!(
            last.ident.to_string().as_str(),
            "Option" | "Vec" | "Map" | "HashMap" | "BTreeMap"
        )
    }) && closed_domain(ty, false)
}

pub(super) fn closed_domain(ty: &syn::Type, unit: bool) -> bool {
    if let syn::Type::Tuple(tuple) = ty {
        return if tuple.elems.is_empty() {
            unit
        } else {
            tuple.elems.iter().all(|t| closed_domain(t, false))
        };
    }
    let syn::Type::Path(path) = ty else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    let name = last.ident.to_string();
    if matches!(name.as_str(), "HashMap" | "BTreeMap") {
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return false;
        };
        let types: Vec<_> = args
            .args
            .iter()
            .filter_map(|a| {
                if let syn::GenericArgument::Type(t) = a {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        return args.args.len() == 2
            && types.len() == 2
            && matches!(types[0],syn::Type::Path(p) if p.path.segments.last().is_some_and(|s|s.ident=="String"))
            && closed_domain(types[1], false);
    }
    if matches!(
        name.as_str(),
        "String" | "str" | "Path" | "PathBuf" | "Value" | "Map" | "HashMap" | "BTreeMap"
    ) {
        return false;
    }
    match &last.arguments {
        syn::PathArguments::None => !matches!(name.as_str(), "Option" | "Vec"),
        syn::PathArguments::AngleBracketed(args) => {
            let exact_wrapper = !matches!(name.as_str(), "Option" | "Vec") || args.args.len() == 1;
            exact_wrapper
                && args.args.iter().all(|arg| match arg {
                    syn::GenericArgument::Type(ty) => closed_domain(ty, false),
                    _ => false,
                })
        }
        _ => false,
    }
}
