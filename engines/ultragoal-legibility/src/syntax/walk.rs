use super::{classify, declarations, imports, visitor::Scanner};
use syn::visit::Visit;

impl<'ast> Visit<'ast> for Scanner {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if declarations::test_only(item) {
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if attr.path().is_ident("derive")
            && let Ok(paths) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
        {
            for path in &paths {
                self.visit_path(path);
            }
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let before = self.conditions.len();
        self.conditions.extend(
            item.attrs
                .iter()
                .filter(|a| a.path().is_ident("cfg"))
                .filter_map(|a| a.meta.require_list().ok().map(|m| m.tokens.to_string())),
        );
        self.enter_function(
            &item.sig,
            !matches!(item.vis, syn::Visibility::Inherited),
            Some(&item.block),
        );
        self.conditions.truncate(before);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let before = self.conditions.len();
        self.conditions.extend(
            item.attrs
                .iter()
                .filter(|a| a.path().is_ident("cfg"))
                .filter_map(|a| a.meta.require_list().ok().map(|m| m.tokens.to_string())),
        );
        self.enter_function(
            &item.sig,
            !matches!(item.vis, syn::Visibility::Inherited),
            Some(&item.block),
        );
        self.conditions.truncate(before);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.enter_function(&item.sig, true, item.default.as_ref());
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.report
            .modules
            .push(self.qualified(&item.ident.to_string()));
        if let Some((_, items)) = &item.content {
            self.owners.push(item.ident.to_string());
            self.scope(items);
            for item in items {
                self.visit_item(item);
            }
            self.leave_scope();
            self.owners.pop();
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.implementation(item);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        self.record_construction(expression);
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.owners.push(item.ident.to_string());
        syn::visit::visit_item_trait(self, item);
        self.owners.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let items = block
            .stmts
            .iter()
            .filter_map(|s| match s {
                syn::Stmt::Item(i) => Some(i.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.scope(&items);
        syn::visit::visit_block(self, block);
        self.leave_scope();
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let closed = !declarations::deserialize(&item.attrs)
            || declarations::has_serde(&item.attrs, "deny_unknown_fields");
        self.record_type(
            &item.ident,
            &item.attrs,
            item.fields.iter().collect(),
            "struct",
            closed,
        );
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        let unit = item
            .variants
            .iter()
            .all(|v| matches!(v.fields, syn::Fields::Unit));
        let closed = unit
            || !declarations::deserialize(&item.attrs)
            || declarations::has_serde(&item.attrs, "deny_unknown_fields");
        self.record_type(
            &item.ident,
            &item.attrs,
            item.variants.iter().flat_map(|v| v.fields.iter()).collect(),
            "enum",
            closed,
        );
        if let Some(declaration) = self.report.types.last_mut() {
            declaration.variants = item.variants.iter().map(|v| v.ident.to_string()).collect();
        }
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.record_alias(item);
    }

    fn visit_type(&mut self, ty: &'ast syn::Type) {
        for kind in classify::type_facts(ty, &self.scopes).1 {
            if self.all_types || self.structured_types && classify::structured(&kind) {
                self.authority(&kind);
            }
        }
        syn::visit::visit_type(self, ty);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        for symbol in imports::resolve(path, &self.scopes) {
            if let Some(kind) = classify::authority(&symbol)
                && !matches!(
                    kind,
                    "string"
                        | "path"
                        | "path_buf"
                        | "serde_json_value"
                        | "serde_json_map"
                        | "toml_value"
                )
            {
                self.authority(kind);
                if kind == "structured_input" {
                    for segment in &path.segments {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let syn::GenericArgument::Type(ty) = arg {
                                    for kind in classify::type_facts(ty, &self.scopes).1 {
                                        self.authority(&kind);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.dependency(symbol);
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        let mut callee = expr.func.as_ref();
        loop {
            callee = match callee {
                syn::Expr::Paren(p) => &p.expr,
                syn::Expr::Group(g) => &g.expr,
                _ => break,
            };
        }
        if let syn::Expr::Path(path) = callee {
            for symbol in imports::resolve(&path.path, &self.scopes) {
                if symbol.starts_with("<local-closure:") {
                    continue;
                }
                if let Some(index) = self.current {
                    self.report.functions[index]
                        .direct_calls
                        .push(symbol.clone());
                }
                self.call(symbol);
            }
        } else if !matches!(callee, syn::Expr::Closure(_))
            && let Some(index) = self.current
        {
            self.report.functions[index]
                .direct_calls
                .push("<indirect-call>".into());
        }
        syn::visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        self.call(expr.method.to_string());
        self.report
            .limitations
            .push("Method calls have no inferred direct-call graph edges".into());
        syn::visit::visit_expr_method_call(self, expr);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        self.inspect_macro(item);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.local_binding(local);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.ident.as_ref().is_some_and(|name| name == "object")
            && declarations::object_contract(&[syn::Item::Macro(item.clone())]) == Some(true)
        {
            return;
        }
        syn::visit::visit_item_macro(self, item);
    }
}
