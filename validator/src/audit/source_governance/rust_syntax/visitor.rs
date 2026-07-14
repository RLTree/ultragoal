use super::adapter::RustSyntaxReport;
use super::classification::{
    authority_for_type, cfg_test, environment_path, process_path, structured_authority,
    structured_parser_path, visible,
};
use super::lint_metadata;
use super::model::AuthorityKind;
use super::use_aliases;
use super::visitor_state::{SyntaxVisitor, TypeContext};
use std::collections::BTreeMap;
use syn::Type;
use syn::visit::Visit;

pub(super) fn inspect(
    file: &syn::File,
    aliases: BTreeMap<String, AuthorityKind>,
    include_test_items: bool,
) -> RustSyntaxReport {
    let mut visitor = SyntaxVisitor::new(aliases, include_test_items);
    visitor.visit_file(file);
    visitor.report()
}

impl<'ast> Visit<'ast> for SyntaxVisitor {
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        match lint_metadata::inspect(&node.meta) {
            Ok(lints) => self.disallowed_lint_allowances.extend(lints),
            Err(failure) => {
                self.lint_metadata_failures.insert(failure.stable_id());
            }
        }
        syn::visit::visit_attribute(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        use_aliases::collect_roots(&node.tree, &mut self.external_roots);
        syn::visit::visit_item_use(self, node);
    }

    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        self.external_roots.insert(node.ident.to_string());
        syn::visit::visit_item_extern_crate(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !self.include_test_items && cfg_test(&node.attrs) {
            return;
        }
        for attribute in &node.attrs {
            self.visit_attribute(attribute);
        }
        self.identifier(&node.sig.ident);
        self.enter_function(
            node.sig.ident.to_string(),
            visible(&node.vis),
            &node.sig,
            Some(&node.block),
        );
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        for attribute in &node.attrs {
            self.visit_attribute(attribute);
        }
        self.identifier(&node.sig.ident);
        self.enter_function(
            node.sig.ident.to_string(),
            visible(&node.vis),
            &node.sig,
            Some(&node.block),
        );
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        for attribute in &node.attrs {
            self.visit_attribute(attribute);
        }
        self.identifier(&node.sig.ident);
        self.enter_function(
            node.sig.ident.to_string(),
            true,
            &node.sig,
            node.default.as_ref(),
        );
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if !self.include_test_items && cfg_test(&node.attrs) {
            return;
        }
        for attribute in &node.attrs {
            self.visit_attribute(attribute);
        }
        self.identifier(&node.ident);
        self.closed_records.insert(node.ident.to_string());
        let prior = self.type_context;
        self.type_context = TypeContext::Structured;
        for field in &node.fields {
            self.visit_field(field);
        }
        self.type_context = prior;
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if !self.include_test_items && cfg_test(&node.attrs) {
            return;
        }
        for attribute in &node.attrs {
            self.visit_attribute(attribute);
        }
        self.identifier(&node.ident);
        self.closed_records.insert(node.ident.to_string());
        let prior = self.type_context;
        self.type_context = TypeContext::Structured;
        for variant in &node.variants {
            self.visit_variant(variant);
        }
        self.type_context = prior;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if self.include_test_items || !cfg_test(&node.attrs) {
            self.identifier(&node.ident);
            syn::visit::visit_item_trait(self, node);
        }
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if self.include_test_items || !cfg_test(&node.attrs) {
            self.identifier(&node.ident);
            syn::visit::visit_item_type(self, node);
        }
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if self.include_test_items || !cfg_test(&node.attrs) {
            self.identifier(&node.ident);
            syn::visit::visit_item_const(self, node);
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if self.include_test_items || !cfg_test(&node.attrs) {
            self.identifier(&node.ident);
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if self.include_test_items || !cfg_test(&node.attrs) {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.identifier(ident);
        }
        syn::visit::visit_field(self, node);
    }

    fn visit_variant(&mut self, node: &'ast syn::Variant) {
        self.identifier(&node.ident);
        syn::visit::visit_variant(self, node);
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.identifier(&node.ident);
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_type(&mut self, node: &'ast Type) {
        for kind in authority_for_type(node, &self.aliases) {
            if matches!(self.type_context, TypeContext::All)
                || matches!(self.type_context, TypeContext::Structured)
                    && structured_authority(kind)
            {
                self.authority(kind);
            }
        }
        syn::visit::visit_type(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        if let Some(first) = node.segments.first() {
            self.external_roots.insert(first.ident.to_string());
        }
        let segments = node
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>();
        let alias_kind = segments
            .first()
            .and_then(|name| self.aliases.get(name))
            .copied();
        if process_path(&segments) || alias_kind == Some(AuthorityKind::Process) {
            self.authority(AuthorityKind::Process);
        }
        if environment_path(&segments) || alias_kind == Some(AuthorityKind::Environment) {
            self.authority(AuthorityKind::Environment);
        }
        if structured_parser_path(&segments) {
            self.structured_parser(node);
        }
        syn::visit::visit_path(self, node);
    }
}
