//! Exact custody-field operations admitted by the reservation owner shape.

use std::collections::BTreeSet;

use syn::visit::Visit;

pub(super) struct BodyShape {
    tracked: BTreeSet<String>,
    current: String,
    references: Vec<String>,
    operations: Vec<String>,
    sensitive_calls: Vec<String>,
    attributes: usize,
    macros: usize,
    unsafe_blocks: usize,
    items: usize,
}

impl BodyShape {
    pub(super) fn new<const N: usize>(tracked: [&str; N]) -> Self {
        Self {
            tracked: tracked.into_iter().map(str::to_owned).collect(),
            current: String::new(),
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

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        if let Some(field) = field(node).filter(|field| self.tracked.contains(field)) {
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
            && sensitive_method(&node.method)
        {
            self.sensitive_calls
                .push(format!("{}:self:{}", self.current, node.method));
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn operation(node: &syn::ExprMethodCall) -> String {
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

fn field(node: &syn::ExprField) -> Option<String> {
    if !matches!(node.base.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) {
        return None;
    }
    match &node.member {
        syn::Member::Named(name) => Some(name.to_string()),
        syn::Member::Unnamed(index) if index.index == 0 => Some("0".to_owned()),
        _ => None,
    }
}

fn sensitive_method(name: &syn::Ident) -> bool {
    [
        "cleanup_staged",
        "finish_terminal",
        "mark_started",
        "record_failure_and_transition",
        "settle_incomplete",
        "settle_success",
        "stage_and_use",
        "clear_recorded",
        "is_empty",
        "push_and_use",
    ]
    .iter()
    .any(|expected| name == expected)
}
