use super::{classify, model::Function, visitor::Scanner};
use quote::ToTokens;
use syn::visit::Visit;

impl Scanner {
    pub(super) fn record_alias(&mut self, item: &syn::ItemType) {
        let name = self.qualified(&item.ident.to_string());
        self.report.types.push(super::TypeDeclaration {
            name,
            closed: false,
            source: "type_alias_unresolved".into(),
            fields: Vec::new(),
            variants: Vec::new(),
            derived_default: false,
            aliased_path: if let syn::Type::Path(p) = item.ty.as_ref() {
                let names = super::imports::resolve(&p.path, &self.scopes);
                (names.len() == 1).then(|| names[0].clone())
            } else {
                None
            },
        });
        syn::visit::visit_item_type(self, item);
    }
    pub(super) fn local_binding(&mut self, local: &syn::Local) {
        syn::visit::visit_local(self, local);
        if let syn::Pat::Ident(binding) = &local.pat {
            let closure = binding.mutability.is_none()
                && local
                    .init
                    .as_ref()
                    .is_some_and(|i| matches!(i.expr.as_ref(), syn::Expr::Closure(_)));
            let name = binding.ident.to_string();
            if let Some(scope) = self.scopes.last_mut() {
                scope.insert(
                    name.clone(),
                    vec![if closure {
                        format!("<local-closure:{name}>")
                    } else {
                        name
                    }],
                );
            }
        }
    }
    pub(super) fn implementation(&mut self, item: &syn::ItemImpl) {
        let resolved = if let syn::Type::Path(ty) = item.self_ty.as_ref() {
            super::imports::resolve(&ty.path, &self.scopes)
        } else {
            vec![]
        };
        self.self_types.push(if resolved.len() == 1 {
            resolved[0].clone()
        } else {
            item.self_ty.to_token_stream().to_string()
        });
        let owner = item.self_ty.to_token_stream().to_string().replace(' ', "");
        let owner = match &item.trait_ {
            Some((tr, _)) => format!(
                "<{owner}as{}>",
                tr.to_token_stream().to_string().replace(' ', "")
            ),
            None => owner,
        };
        self.owners.push(owner);
        syn::visit::visit_item_impl(self, item);
        self.owners.pop();
        self.self_types.pop();
    }
    pub(super) fn record_construction(&mut self, expression: &syn::ExprStruct) {
        if let Some(index) = self.current {
            self.report.functions[index]
                .constructions
                .extend(super::imports::resolve(&expression.path, &self.scopes));
        }
    }
    pub(super) fn enter_function(
        &mut self,
        sig: &syn::Signature,
        visible: bool,
        body: Option<&syn::Block>,
    ) {
        let prior = self.current;
        let prior_types = self.all_types;
        let prior_structured = self.structured_types;
        let name = self.qualified(&sig.ident.to_string());
        let expanded =
            super::type_aliases::expand_output(&sig.output, &self.scopes, &self.type_aliases);
        if let Err(error) = &expanded {
            self.report.limitations.push(format!("{name}: {error}"));
        }
        let effective = expanded.as_ref().unwrap_or(&sig.output);
        let (return_type, output_identifiers) = match &sig.output {
            syn::ReturnType::Default => ("()".into(), Vec::new()),
            syn::ReturnType::Type(_, ty) => (
                ty.to_token_stream().to_string(),
                super::type_aliases::output_paths(effective, &self.scopes),
            ),
        };
        let input_types = sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(arg) => Some(arg.ty.to_token_stream().to_string()),
                _ => None,
            })
            .collect();
        self.current = Some(self.report.functions.len());
        self.report.functions.push(Function {
            name,
            return_type,
            returns_closed_result: expanded.is_ok()
                && classify::closed_result(effective, &self.scopes),
            resolved_error: classify::resolved_error(effective, &self.scopes),
            output_identifiers,
            input_types,
            self_type: self.self_types.last().cloned(),
            conditions: self.conditions.clone(),
            generic: super::generic_bounds::collect(sig, effective, &self.scopes),
            ..Function::default()
        });
        self.all_types = visible || classify::semantic(&sig.ident.to_string());
        self.structured_types = true;
        self.visit_signature(sig);
        self.all_types = false;
        self.structured_types = false;
        self.owners.push(sig.ident.to_string());
        if let Some(body) = body {
            self.visit_block(body);
        }
        self.owners.pop();
        self.current = prior;
        self.all_types = prior_types;
        self.structured_types = prior_structured;
    }
}
