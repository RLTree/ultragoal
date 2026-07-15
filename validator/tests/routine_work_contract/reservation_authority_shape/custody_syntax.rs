pub(super) fn operation(node: &syn::ExprMethodCall) -> String {
    if node.method != "set" {
        return node.method.to_string();
    }
    if matches!(node.args.first(), Some(syn::Expr::Lit(lit)) if matches!(&lit.lit, syn::Lit::Bool(value) if value.value))
        && node.args.len() == 1
    {
        "set(true)".to_owned()
    } else {
        "set(other)".to_owned()
    }
}

pub(super) fn field(node: &syn::ExprField) -> Option<String> {
    if !matches!(node.base.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) {
        return None;
    }
    match &node.member {
        syn::Member::Named(name) => Some(name.to_string()),
        syn::Member::Unnamed(index) if index.index == 0 => Some("0".to_owned()),
        _ => None,
    }
}

pub(super) fn direct_sensitive_method(name: &syn::Ident) -> bool {
    [
        "cleanup_staged",
        "clear_recorded",
        "finish_terminal",
        "mark_started",
        "push_and_use",
        "record_failure_and_transition",
        "settle_incomplete",
        "settle_success",
        "stage_and_use",
    ]
    .iter()
    .any(|expected| name == expected)
}

pub(super) fn contains_owner(
    expression: &syn::Expr,
    owners: &std::collections::BTreeSet<String>,
) -> bool {
    struct Find<'a> {
        owners: &'a std::collections::BTreeSet<String>,
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for Find<'_> {
        fn visit_path(&mut self, node: &'ast syn::Path) {
            self.found |= node
                .segments
                .last()
                .is_some_and(|part| self.owners.contains(&part.ident.to_string()));
            syn::visit::visit_path(self, node);
        }
    }
    let mut find = Find {
        owners,
        found: false,
    };
    syn::visit::Visit::visit_expr(&mut find, expression);
    find.found
}

pub(super) fn allowed_external_method(node: &syn::ExprMethodCall) -> bool {
    node.method == "cleanup_staged"
        && matches!(node.receiver.as_ref(), syn::Expr::Field(field)
            if matches!(field.base.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self"))
                && matches!(&field.member, syn::Member::Named(member) if member == "durable"))
}

pub(super) fn allowed_external_transition(path: &syn::Path) -> bool {
    matches!(
        path_name(path).as_str(),
        "registry_transition::finish_terminal" | "registry_transition::mark_started"
    )
}

pub(super) fn sensitive_path(path: &syn::Path) -> bool {
    path.segments.last().is_some_and(|segment| {
        direct_sensitive_method(&segment.ident) || segment.ident == "is_empty"
    })
}

pub(super) fn path_name(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
