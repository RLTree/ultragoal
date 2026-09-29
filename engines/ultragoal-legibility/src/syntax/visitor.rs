use super::{classify, declarations, imports, model::*};
use quote::ToTokens;
use syn::visit::Visit;

#[derive(Default)]
pub(super) struct Scanner {
    pub(super) report: Report,
    pub(super) scopes: Vec<imports::Aliases>,
    pub(super) type_aliases: Vec<super::type_aliases::Definitions>,
    pub(super) owners: Vec<String>,
    pub(super) self_types: Vec<String>,
    pub(super) conditions: Vec<String>,
    pub(super) current: Option<usize>,
    pub(super) all_types: bool,
    pub(super) structured_types: bool,
    pub(super) declaration: Option<String>,
    pub(super) object_contract: Vec<Option<bool>>,
    pub(super) type_fields: Vec<(String, Vec<String>)>,
}

impl Scanner {
    pub(super) fn qualified(&self, name: &str) -> String {
        self.owners
            .iter()
            .map(String::as_str)
            .chain([name])
            .collect::<Vec<_>>()
            .join("::")
    }

    pub(super) fn function_name(&self) -> Option<String> {
        self.current.map(|i| self.report.functions[i].name.clone())
    }

    pub(super) fn scope(&mut self, items: &[syn::Item]) {
        let (aliases, paths) = imports::collect(items);
        self.scopes.push(aliases);
        self.type_aliases.push(super::type_aliases::collect(items));
        self.object_contract
            .push(declarations::object_contract(items));
        for path in paths {
            if path.ends_with("::*") {
                self.report
                    .limitations
                    .push(format!("Unresolved glob import: {path}"));
            }
        }
    }

    pub(super) fn leave_scope(&mut self) {
        self.scopes.pop();
        self.type_aliases.pop();
        self.object_contract.pop();
    }

    pub(super) fn dependency(&mut self, symbol: String) {
        let root = symbol.split("::").next().unwrap_or("");
        if matches!(root, "crate" | "self" | "super" | "Self") {
            return;
        }
        self.report.dependencies.push(DependencyUse {
            crate_name: root.into(),
            symbol,
            function: self.function_name(),
            owner: self.function_name().or_else(|| self.declaration.clone()),
        });
    }

    pub(super) fn authority(&mut self, kind: &str) {
        self.report.authorities.push(AuthorityUse {
            function: self.function_name(),
            owner: self.function_name().or_else(|| self.declaration.clone()),
            kind: kind.into(),
        });
        if let Some(index) = self.current {
            self.report.functions[index]
                .authority_kinds
                .push(kind.into());
        }
    }

    pub(super) fn call(&mut self, symbol: String) {
        if let Some(index) = self.current {
            let function = &mut self.report.functions[index];
            function.calls.push(symbol.clone());
            function.validation_calls.push(symbol);
        }
    }

    pub(super) fn record_type(
        &mut self,
        name: &syn::Ident,
        attrs: &[syn::Attribute],
        fields: Vec<&syn::Field>,
        source: &str,
        base_closed: bool,
    ) {
        let prior = self.declaration.replace(self.qualified(&name.to_string()));
        let prior_structured = self.structured_types;
        self.structured_types = true;
        for attribute in attrs {
            self.visit_attribute(attribute);
        }
        if source == "object_macro" && base_closed {
            for derive in ["Serialize", "Deserialize"] {
                self.dependency(format!("serde::{derive}"));
            }
        }
        let mut closed = base_closed && !declarations::has_serde(attrs, "flatten");
        let mut referenced = Vec::new();
        let field_shapes = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                (
                    field
                        .ident
                        .as_ref()
                        .map_or_else(|| index.to_string(), ToString::to_string),
                    field.ty.to_token_stream().to_string(),
                )
            })
            .collect();
        for field in fields {
            let (paths, kinds) = classify::type_facts(&field.ty, &self.scopes);
            closed &= !kinds.iter().any(|kind| classify::structured(kind));
            closed &= !declarations::has_serde(&field.attrs, "flatten");
            referenced.extend(paths);
            self.visit_field(field);
        }
        let name = self.qualified(&name.to_string());
        self.type_fields.push((name.clone(), referenced));
        self.report.types.push(TypeDeclaration {
            name,
            closed,
            source: source.into(),
            fields: field_shapes,
            variants: Vec::new(),
            derived_default: attrs
                .iter()
                .filter(|a| a.path().is_ident("derive"))
                .any(|a| {
                    a.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                    )
                    .is_ok_and(|paths| {
                        paths
                            .iter()
                            .any(|p| imports::resolve(p, &self.scopes) == ["std::default::Default"])
                    })
                }),
            aliased_path: None,
        });
        self.declaration = prior;
        self.structured_types = prior_structured;
    }

    pub(super) fn finish(mut self) -> Report {
        loop {
            let open = self
                .report
                .types
                .iter()
                .filter(|ty| !ty.closed)
                .map(|ty| ty.name.clone())
                .collect::<Vec<_>>();
            let mut changed = false;
            for (name, fields) in &self.type_fields {
                if fields.iter().any(|field| {
                    open.iter()
                        .any(|o| o == field || o == &self.sibling(name, field))
                }) {
                    for ty in &mut self.report.types {
                        if &ty.name == name && ty.closed {
                            ty.closed = false;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        self.report.dependencies.sort();
        self.report.dependencies.dedup();
        self.report.authorities.sort();
        self.report.authorities.dedup();
        self.report.limitations.sort();
        self.report.limitations.dedup();
        for function in &mut self.report.functions {
            function.calls.sort();
            function.calls.dedup();
            function.direct_calls.sort();
            function.direct_calls.dedup();
            function.validation_calls.sort();
            function.validation_calls.dedup();
            function.authority_kinds.sort();
            function.authority_kinds.dedup();
        }
        self.report
    }

    fn sibling(&self, name: &str, field: &str) -> String {
        name.rsplit_once("::")
            .map_or_else(|| field.into(), |(scope, _)| format!("{scope}::{field}"))
    }
}
