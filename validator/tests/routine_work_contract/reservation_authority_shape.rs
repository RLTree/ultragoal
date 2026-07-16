use syn::visit::Visit;

pub(super) fn validate(
    authority: &str,
    wrapper: &str,
    attempt: &str,
    registry: &str,
    state: &str,
    mediator: &str,
) -> Result<(), &'static str> {
    let authority_file = parse(authority, "authority-parse")?;
    let wrapper_file = parse(wrapper, "owner-wrapper-parse")?;
    let attempt_file = parse(attempt, "owner-parse")?;
    let registry_file = parse(registry, "registry-parse")?;
    exact_modules(
        &authority_file,
        &[
            ("binding", "authority/binding.rs"),
            ("durable_binding", "authority/durable_binding.rs"),
            ("failure_observation", "authority/failure_observation.rs"),
            ("owner", "authority/owner.rs"),
        ],
        "authority-module-paths",
    )?;
    exact_modules(
        &wrapper_file,
        &[
            ("attempt", "owner/attempt.rs"),
            ("registry", "owner/registry.rs"),
        ],
        "owner-module-paths",
    )?;
    leaf(attempt, &attempt_file, 0, "owner-leaf")?;
    leaf(registry, &registry_file, 1, "registry-leaf")?;
    owner_surface(&attempt_file)?;
    registry_surface(&registry_file)?;
    if state.contains("AttemptReservation") || state.contains("registry") {
        return Err("state-raw-export");
    }
    if mediator.contains("pub(crate) use reservation_state") {
        return Err("mediator-raw-export");
    }
    Ok(())
}

fn parse(source: &str, error: &'static str) -> Result<syn::File, &'static str> {
    syn::parse_file(source).map_err(|_| error)
}

fn exact_modules(
    file: &syn::File,
    expected: &[(&str, &str)],
    error: &'static str,
) -> Result<(), &'static str> {
    let mut actual = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) => Some((module.ident.to_string(), module_path(module)?)),
            _ => None,
        })
        .collect::<Vec<_>>();
    actual.sort();
    let expected = expected
        .iter()
        .map(|(name, path)| ((*name).to_owned(), (*path).to_owned()))
        .collect::<Vec<_>>();
    (actual == expected).then_some(()).ok_or(error)
}

fn module_path(module: &syn::ItemMod) -> Option<String> {
    module.attrs.iter().find_map(|attribute| {
        let syn::Meta::NameValue(value) = &attribute.meta else {
            return None;
        };
        let syn::Expr::Lit(expression) = &value.value else {
            return None;
        };
        match (&expression.lit, attribute.path().is_ident("path")) {
            (syn::Lit::Str(value), true) => Some(value.value()),
            _ => None,
        }
    })
}

fn leaf(
    source: &str,
    file: &syn::File,
    cfg_count: usize,
    error: &'static str,
) -> Result<(), &'static str> {
    let mut hidden = HiddenCode(false);
    hidden.visit_file(file);
    let trait_impl = file
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Impl(item) if item.trait_.is_some()));
    (!hidden.0 && !trait_impl && source.matches("#[cfg").count() == cfg_count)
        .then_some(())
        .ok_or(error)
}

struct HiddenCode(bool);

impl<'ast> Visit<'ast> for HiddenCode {
    fn visit_item_mod(&mut self, _: &'ast syn::ItemMod) {
        self.0 = true;
    }
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.0 = true;
    }
    fn visit_expr_unsafe(&mut self, _: &'ast syn::ExprUnsafe) {
        self.0 = true;
    }
    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if matches!(statement, syn::Stmt::Item(_)) {
            self.0 = true;
        } else {
            syn::visit::visit_stmt(self, statement);
        }
    }
}

fn owner_surface(file: &syn::File) -> Result<(), &'static str> {
    private_struct(file, "AttemptReservation")?;
    let capability = struct_item(file, "ReservationAttempt")?;
    if vis(&capability.vis) != 4 || capability.fields.iter().any(|field| vis(&field.vis) != 0) {
        return Err("owner-capability-visible");
    }
    exact_methods(
        file,
        "AttemptReservation",
        0,
        &[
            "capture_staged_cleanup",
            "cleanup_staged",
            "record_failure",
            "require_open",
            "require_ready",
            "reserve",
            "settle_terminal",
        ],
        "owner-raw-methods",
    )?;
    exact_methods(
        file,
        "ReservationAttempt",
        4,
        &[
            "authenticates_artifact",
            "mark_started",
            "prepare_spawn",
            "retain_non_durable_authentication",
            "reuse_only",
            "stage_and_use",
            "stage_success",
        ],
        "owner-capability-methods",
    )
}

fn registry_surface(file: &syn::File) -> Result<(), &'static str> {
    private_struct(file, "RegistryState")?;
    let mut commands = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function) if vis(&function.vis) != 0 => {
                Some(format!("{}:{}", function.sig.ident, vis(&function.vis)))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    commands.sort();
    (commands
        == [
            "authenticates_non_durable:1",
            "finish_terminal:1",
            "mark_started:1",
            "observe:4",
            "record_failure_and_transition:1",
            "reserve:1",
            "retain_non_durable:1",
        ])
    .then_some(())
    .ok_or("registry-command-surface")
}

fn private_struct(file: &syn::File, name: &str) -> Result<(), &'static str> {
    let item = struct_item(file, name)?;
    (vis(&item.vis) == 0 && item.fields.iter().all(|field| vis(&field.vis) == 0))
        .then_some(())
        .ok_or("raw-storage-visible")
}

fn struct_item<'a>(file: &'a syn::File, name: &str) -> Result<&'a syn::ItemStruct, &'static str> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == name => Some(item),
            _ => None,
        })
        .ok_or("owner-struct-missing")
}

fn exact_methods(
    file: &syn::File,
    owner: &str,
    expected_vis: usize,
    expected: &[&str],
    error: &'static str,
) -> Result<(), &'static str> {
    let mut actual = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) if type_name(&item.self_ty).as_deref() == Some(owner) => {
                Some(item)
            }
            _ => None,
        })
        .flat_map(|item| item.items.iter())
        .filter_map(|member| match member {
            syn::ImplItem::Fn(method) if vis(&method.vis) == expected_vis => {
                Some(method.sig.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    actual.sort();
    (actual == expected).then_some(()).ok_or(error)
}

fn type_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|part| part.ident.to_string())
}

fn vis(value: &syn::Visibility) -> usize {
    match value {
        syn::Visibility::Inherited => 0,
        syn::Visibility::Restricted(value) => value.path.segments.len(),
        _ => usize::MAX,
    }
}
