pub(super) fn validate(
    authority: &syn::File,
    custody: &syn::File,
    staged: &syn::File,
    flag: &syn::File,
) -> Result<(), &'static str> {
    validate_function(authority, f("reserve_grant", "super::super", 1))?;
    validate_impl(authority, "AttemptReservation", AUTHORITY)?;
    validate_impl(custody, "AttemptCustody", CUSTODY)?;
    validate_impl(staged, "StagedCustody", STAGED)?;
    validate_impl(flag, "TransitionFlag", FLAG)?;
    validate_private_functions(custody)
}

#[derive(Clone, Copy)]
struct Spec {
    name: &'static str,
    visibility: &'static str,
    arguments: usize,
    generics: usize,
    receiver: bool,
}

const fn f(name: &'static str, visibility: &'static str, arguments: usize) -> Spec {
    Spec {
        name,
        visibility,
        arguments,
        generics: 0,
        receiver: false,
    }
}

const fn m(name: &'static str, visibility: &'static str, arguments: usize) -> Spec {
    Spec {
        name,
        visibility,
        arguments,
        generics: 0,
        receiver: true,
    }
}

const fn g(name: &'static str, visibility: &'static str, arguments: usize) -> Spec {
    Spec {
        generics: 1,
        ..m(name, visibility, arguments)
    }
}

const AUTHORITY: &[Spec] = &[
    m("protocol_id", "super::super", 0),
    m("grant_id", "super::super", 0),
    m("recovery_marker", "super::super", 0),
    m("is_started", "super::super", 0),
    m("terminal_is_authoritative", "super", 0),
    m("reuse_only", "super::super", 0),
    m("prepare_spawn", "super::super", 0),
    m("mark_started", "super::super", 0),
    m("stage_success", "super::super", 1),
    m("settle_success", "super::super", 1),
    m("settle_incomplete", "super::super", 1),
    m("retain_non_durable_authentication", "super::super", 1),
    m("authenticates_artifact", "super::super", 2),
    g("stage_and_use", "super::super", 2),
    m("cleanup_staged", "super", 0),
    m("failure_evidence", "super", 3),
    m("record_failure_and_transition", "super", 1),
];
const CUSTODY: &[Spec] = &[
    f("new", "super", 0),
    m("is_started", "super", 0),
    m("terminal_is_authoritative", "super", 0),
    m("require_open", "super", 0),
    m("mark_started", "super", 1),
    m("finish_terminal", "super", 2),
    m("require_staged_empty", "super", 0),
    g("stage_and_use", "super", 2),
    m("cleanup_staged", "super", 2),
    m("failure_transfer_required", "super", 2),
    m("finish_failure", "super", 2),
];
const STAGED: &[Spec] = &[
    f("new", "super", 0),
    m("is_empty", "super", 0),
    m("require_empty", "super", 0),
    g("push_and_use", "super", 2),
    m("cleanup_last", "super", 1),
    m("clear_recorded", "super", 0),
    m("failure_transfer_required", "super", 2),
];
const FLAG: &[Spec] = &[
    f("new", "super", 0),
    m("is_set", "super", 0),
    m("mark", "super", 0),
];

fn validate_impl(file: &syn::File, owner: &str, expected: &[Spec]) -> Result<(), &'static str> {
    let implementations = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) if type_is(&item.self_ty, owner) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [implementation] = implementations.as_slice() else {
        return Err("authority-impl-shape");
    };
    if !implementation.attrs.is_empty()
        || implementation.trait_.is_some()
        || implementation.unsafety.is_some()
        || !implementation.generics.params.is_empty()
    {
        return Err("authority-impl-shape");
    }
    let methods = implementation
        .items
        .iter()
        .map(|item| match item {
            syn::ImplItem::Fn(method) => Ok(method),
            _ => Err("authority-impl-member-kind"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if methods.len() != expected.len() {
        return Err("authority-methods");
    }
    for spec in expected {
        let Some(method) = methods.iter().find(|method| method.sig.ident == spec.name) else {
            return Err("authority-methods");
        };
        validate_signature(&method.attrs, &method.vis, &method.sig, *spec)?;
    }
    Ok(())
}

fn validate_function(file: &syn::File, spec: Spec) -> Result<(), &'static str> {
    let Some(function) = file.items.iter().find_map(|item| match item {
        syn::Item::Fn(function) if function.sig.ident == spec.name => Some(function),
        _ => None,
    }) else {
        return Err("authority-function-missing");
    };
    validate_signature(&function.attrs, &function.vis, &function.sig, spec)
}

fn validate_private_functions(file: &syn::File) -> Result<(), &'static str> {
    let expected = [
        f("clear_exact_ambiguity", "private", 3),
        f("clear_exact_failure", "private", 2),
    ];
    let functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function) => Some(function),
            _ => None,
        })
        .collect::<Vec<_>>();
    if functions.len() != expected.len() {
        return Err("authority-functions");
    }
    for spec in expected {
        let Some(function) = functions
            .iter()
            .find(|function| function.sig.ident == spec.name)
        else {
            return Err("authority-functions");
        };
        validate_signature(&function.attrs, &function.vis, &function.sig, spec)?;
    }
    Ok(())
}

fn validate_signature(
    attrs: &[syn::Attribute],
    visibility: &syn::Visibility,
    signature: &syn::Signature,
    spec: Spec,
) -> Result<(), &'static str> {
    if !attrs.is_empty()
        || visibility_name(visibility) != spec.visibility
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.generics.params.len() != spec.generics
        || signature.generics.where_clause.is_some()
    {
        return Err("authority-method-signature");
    }
    let mut inputs = signature.inputs.iter();
    if spec.receiver {
        let Some(syn::FnArg::Receiver(receiver)) = inputs.next() else {
            return Err("authority-method-signature");
        };
        if receiver.reference.is_none()
            || receiver.mutability.is_some()
            || receiver.colon_token.is_some()
        {
            return Err("authority-method-signature");
        }
    }
    let arguments = inputs.collect::<Vec<_>>();
    if arguments.len() != spec.arguments
        || arguments.iter().any(|argument| match argument {
            syn::FnArg::Typed(argument) => !matches!(argument.pat.as_ref(), syn::Pat::Ident(_)),
            syn::FnArg::Receiver(_) => true,
        })
    {
        return Err("authority-method-signature");
    }
    Ok(())
}

fn type_is(value: &syn::Type, expected: &str) -> bool {
    matches!(value, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(expected))
}

fn visibility_name(value: &syn::Visibility) -> String {
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
