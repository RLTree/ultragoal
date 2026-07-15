use syn::visit::Visit;

pub(super) fn validate(
    authority: &syn::File,
    custody: &syn::File,
    staged: &syn::File,
    flag: &syn::File,
) -> Result<(), &'static str> {
    validate_authority(authority)?;
    validate_custody(custody)?;
    validate_tuple_leaf(staged, "StagedCustody", "RefCell")?;
    validate_tuple_leaf(flag, "TransitionFlag", "Cell")
}

fn validate_authority(file: &syn::File) -> Result<(), &'static str> {
    let modules = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) => Some(module),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut names = modules
        .iter()
        .map(|module| module.ident.to_string())
        .collect::<Vec<_>>();
    names.sort();
    if !file.attrs.is_empty()
        || names != ["custody", "staged_custody", "transition_flag"]
        || modules.iter().any(|module| {
            module.content.is_some()
                || module.attrs.len() != 1
                || !module.attrs[0].path().is_ident("path")
        })
    {
        return Err("authority-module-boundary");
    }
    if file.items.iter().any(|item| match item {
        syn::Item::Use(_) | syn::Item::Mod(_) => false,
        syn::Item::Struct(item) => item.ident != "AttemptReservation",
        syn::Item::Fn(item) => item.sig.ident != "reserve_grant",
        syn::Item::Impl(item) => !type_is(&item.self_ty, "AttemptReservation"),
        _ => true,
    }) {
        return Err("authority-top-level-items");
    }
    validate_named_owner(
        file,
        "AttemptReservation",
        "super::super",
        &[
            ("binding", "ReservationBinding"),
            ("custody", "AttemptCustody"),
            ("durable", "DurableBinding"),
        ],
    )?;
    validate_constructor(file)
}

fn validate_custody(file: &syn::File) -> Result<(), &'static str> {
    if !file.attrs.is_empty()
        || file.items.iter().any(|item| match item {
            syn::Item::Use(_) => false,
            syn::Item::Struct(item) => item.ident != "AttemptCustody",
            syn::Item::Impl(item) => !type_is(&item.self_ty, "AttemptCustody"),
            syn::Item::Fn(item) => !["clear_exact_ambiguity", "clear_exact_failure"]
                .iter()
                .any(|name| item.sig.ident == name),
            _ => true,
        })
    {
        return Err("authority-leaf-items");
    }
    validate_named_owner(
        file,
        "AttemptCustody",
        "super",
        &[
            ("settled", "TransitionFlag"),
            ("staged", "StagedCustody"),
            ("started", "TransitionFlag"),
        ],
    )
}

fn validate_tuple_leaf(
    file: &syn::File,
    owner: &str,
    expected_type: &str,
) -> Result<(), &'static str> {
    if !file.attrs.is_empty()
        || file.items.iter().any(|item| match item {
            syn::Item::Use(_) => false,
            syn::Item::Struct(item) => item.ident != owner,
            syn::Item::Impl(item) => !type_is(&item.self_ty, owner),
            _ => true,
        })
    {
        return Err("authority-leaf-items");
    }
    let item = owner_item(file, owner)?;
    let syn::Fields::Unnamed(fields) = &item.fields else {
        return Err("authority-leaf-storage");
    };
    if fields.unnamed.len() != 1 {
        return Err("authority-leaf-storage");
    }
    let field = fields.unnamed.first().ok_or("authority-leaf-storage")?;
    if !item.attrs.is_empty()
        || visibility(&item.vis) != "super"
        || !matches!(field.vis, syn::Visibility::Inherited)
        || type_name(&field.ty).as_deref() != Some(expected_type)
    {
        return Err("authority-leaf-storage");
    }
    Ok(())
}

fn validate_named_owner(
    file: &syn::File,
    owner: &str,
    expected_visibility: &str,
    expected: &[(&str, &str)],
) -> Result<(), &'static str> {
    let item = owner_item(file, owner)?;
    if !item.attrs.is_empty()
        || visibility(&item.vis) != expected_visibility
        || item.fields.len() != expected.len()
    {
        return Err("authority-owner-shape");
    }
    for field in &item.fields {
        if !matches!(field.vis, syn::Visibility::Inherited) {
            return Err("authority-field-visible");
        }
        let name = field.ident.as_ref().ok_or("authority-field-shape")?;
        let ty = type_name(&field.ty).ok_or("authority-field-type")?;
        if !expected
            .iter()
            .any(|(expected_name, expected_type)| name == expected_name && ty == *expected_type)
        {
            return Err("authority-fields");
        }
    }
    Ok(())
}

fn validate_constructor(file: &syn::File) -> Result<(), &'static str> {
    let mut constructors = Vec::new();
    ConstructorFind(&mut constructors).visit_file(file);
    let [constructor] = constructors.as_slice() else {
        return Err("authority-constructor-count");
    };
    let fields = constructor
        .fields
        .iter()
        .map(|field| member_name(&field.member))
        .collect::<Vec<_>>();
    if constructor.rest.is_some()
        || !["binding", "custody", "durable"]
            .iter()
            .all(|name| fields.iter().any(|field| field == name))
        || constructor.fields.iter().any(|field| {
            member_name(&field.member) == "custody"
                && !call_is(&field.expr, &["AttemptCustody", "new"])
        })
    {
        return Err("authority-constructor-shape");
    }
    Ok(())
}

struct ConstructorFind<'a>(&'a mut Vec<syn::ExprStruct>);

impl Visit<'_> for ConstructorFind<'_> {
    fn visit_expr_struct(&mut self, expression: &syn::ExprStruct) {
        if expression
            .path
            .segments
            .last()
            .is_some_and(|part| part.ident == "AttemptReservation")
        {
            self.0.push(expression.clone());
        }
        syn::visit::visit_expr_struct(self, expression);
    }
}

fn owner_item<'a>(file: &'a syn::File, owner: &str) -> Result<&'a syn::ItemStruct, &'static str> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == owner => Some(item),
            _ => None,
        })
        .ok_or("authority-owner-missing")
}

fn call_is(expression: &syn::Expr, expected: &[&str]) -> bool {
    let syn::Expr::Call(call) = expression else {
        return false;
    };
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    path.path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .eq(expected.iter().copied())
}

fn type_is(value: &syn::Type, expected: &str) -> bool {
    type_name(value).as_deref() == Some(expected)
}

fn type_name(value: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = value else {
        return None;
    };
    path.path.segments.last().map(|part| part.ident.to_string())
}

fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(name) => name.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
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
