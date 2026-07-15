//! Exact custody-field operations admitted by the reservation owner shape.

use std::collections::BTreeSet;

use super::custody_syntax::{
    allowed_external_method, allowed_external_transition, contains_owner, direct_sensitive_method,
    field, operation, path_name, sensitive_path,
};
use syn::visit::Visit;

pub(super) struct BodyShape {
    tracked: BTreeSet<String>,
    forbidden_patterns: BTreeSet<String>,
    patterns: Vec<String>,
    current: String,
    pub(super) self_paths: Vec<String>,
    pub(super) direct_self_paths: Vec<String>,
    pub(super) constructor_locals: usize,
    pub(super) hidden_custody: usize,
    pub(super) indirect_transitions: Vec<String>,
    pub(super) import_renames: usize,
    closure_depth: usize,
    references: Vec<String>,
    operations: Vec<String>,
    sensitive_calls: Vec<String>,
    attributes: usize,
    macros: usize,
    unsafe_blocks: usize,
    items: usize,
}
impl BodyShape {
    pub(super) fn new<const N: usize, const M: usize>(
        tracked: [&str; N],
        forbidden_patterns: [&str; M],
    ) -> Self {
        Self {
            tracked: tracked.into_iter().map(str::to_owned).collect(),
            forbidden_patterns: forbidden_patterns.into_iter().map(str::to_owned).collect(),
            patterns: Vec::new(),
            current: String::new(),
            self_paths: Vec::new(),
            direct_self_paths: Vec::new(),
            constructor_locals: 0,
            hidden_custody: 0,
            indirect_transitions: Vec::new(),
            import_renames: 0,
            closure_depth: 0,
            references: Vec::new(),
            operations: Vec::new(),
            sensitive_calls: Vec::new(),
            attributes: 0,
            macros: 0,
            unsafe_blocks: 0,
            items: 0,
        }
    }
    pub(super) fn require_plain(&self, top_level_items: usize) -> Result<(), &'static str> {
        (self.attributes == 0
            && self.macros == 0
            && self.unsafe_blocks == 0
            && self.items == top_level_items)
            .then_some(())
            .ok_or("authority-build-specific-or-hidden-code")
    }
    pub(super) fn require_bound_operations<const N: usize>(
        &self,
        expected: [&str; N],
    ) -> Result<(), &'static str> {
        let mut operations = self.operations.clone();
        operations.sort();
        let mut references = self.references.clone();
        references.sort();
        let bound_references = operations
            .iter()
            .map(|entry| {
                entry
                    .rsplit_once(':')
                    .expect("bound operation")
                    .0
                    .to_owned()
            })
            .collect::<Vec<_>>();
        if bound_references != references {
            return Err("authority-unbound-custody-reference");
        }
        let mut expected = expected.into_iter().map(str::to_owned).collect::<Vec<_>>();
        expected.sort();
        (operations == expected)
            .then_some(())
            .ok_or("authority-custody-operations")
    }
    pub(super) fn require_no_custody_patterns(&self) -> Result<(), &'static str> {
        self.patterns
            .is_empty()
            .then_some(())
            .ok_or("authority-custody-pattern-alias")
    }
    pub(super) fn require_sensitive_calls<const N: usize>(
        &self,
        expected: [&str; N],
    ) -> Result<(), &'static str> {
        let mut actual = self.sensitive_calls.clone();
        actual.sort();
        let mut expected = expected.into_iter().map(str::to_owned).collect::<Vec<_>>();
        expected.sort();
        (actual == expected)
            .then_some(())
            .ok_or("authority-transition-calls")
    }
}

impl<'ast> Visit<'ast> for BodyShape {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.items += 1;
        syn::visit::visit_item(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prior = std::mem::replace(&mut self.current, node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current = prior;
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.attributes += 1;
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.macros += 1;
    }

    fn visit_expr_unsafe(&mut self, _: &'ast syn::ExprUnsafe) {
        self.unsafe_blocks += 1;
    }

    fn visit_use_rename(&mut self, node: &'ast syn::UseRename) {
        self.import_renames += 1;
        syn::visit::visit_use_rename(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node.path.is_ident("self") {
            self.self_paths.push(self.current.clone());
        }
        if sensitive_path(&node.path) {
            self.indirect_transitions
                .push(format!("{}:{}", self.current, path_name(&node.path)));
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.closure_depth += 1;
        syn::visit::visit_expr_closure(self, node);
        self.closure_depth -= 1;
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        self.closure_depth += 1;
        syn::visit::visit_expr_async(self, node);
        self.closure_depth -= 1;
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if node
            .init
            .as_ref()
            .is_some_and(|init| contains_owner(&init.expr, &self.forbidden_patterns))
        {
            self.constructor_locals += 1;
        }
        syn::visit::visit_local(self, node);
    }

    fn visit_pat_struct(&mut self, node: &'ast syn::PatStruct) {
        self.record_custody_pattern(&node.path);
        syn::visit::visit_pat_struct(self, node);
    }

    fn visit_pat_tuple_struct(&mut self, node: &'ast syn::PatTupleStruct) {
        self.record_custody_pattern(&node.path);
        syn::visit::visit_pat_tuple_struct(self, node);
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        if matches!(node.base.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) {
            self.direct_self_paths.push(self.current.clone());
        }
        if let Some(field) = field(node).filter(|field| self.tracked.contains(field)) {
            if self.closure_depth > 0 {
                self.hidden_custody += 1;
            }
            self.references.push(format!("{}:{field}", self.current));
        }
        syn::visit::visit_expr_field(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let syn::Expr::Field(receiver) = node.receiver.as_ref() {
            if let Some(field) = field(receiver).filter(|field| self.tracked.contains(field)) {
                self.operations
                    .push(format!("{}:{field}:{}", self.current, operation(node)));
            }
        } else if matches!(node.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self"))
        {
            self.direct_self_paths.push(self.current.clone());
            if direct_sensitive_method(&node.method) || node.method == "is_empty" {
                if self.closure_depth > 0 {
                    self.hidden_custody += 1;
                }
                self.sensitive_calls
                    .push(format!("{}:self:{}", self.current, node.method));
            }
        } else if direct_sensitive_method(&node.method) && !allowed_external_method(node) {
            self.indirect_transitions
                .push(format!("{}:{}", self.current, node.method));
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if sensitive_path(&path.path) {
                if !allowed_external_transition(&path.path) {
                    self.indirect_transitions.push(format!(
                        "{}:{}",
                        self.current,
                        path_name(&path.path)
                    ));
                }
                for arg in &node.args {
                    self.visit_expr(arg);
                }
                return;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

impl BodyShape {
    fn record_custody_pattern(&mut self, path: &syn::Path) {
        let Some(segment) = path.segments.last() else {
            return;
        };
        if self.forbidden_patterns.contains(&segment.ident.to_string()) {
            self.patterns.push(segment.ident.to_string());
        }
    }
}
