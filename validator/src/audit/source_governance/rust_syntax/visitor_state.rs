use super::adapter::RustSyntaxReport;
use super::classification::{authority_for_type, closed_result, semantic_boundary_symbol};
use super::model::{AuthorityKind, AuthorityUse, FunctionShape, IdentifierDeclaration};
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::Visit;
use syn::{GenericArgument, PathArguments};

pub(super) struct SyntaxVisitor {
    pub(super) aliases: BTreeMap<String, AuthorityKind>,
    authorities: BTreeSet<AuthorityUse>,
    pub(super) closed_records: BTreeSet<String>,
    pub(super) disallowed_lint_allowances: BTreeSet<String>,
    pub(super) external_roots: BTreeSet<String>,
    functions: Vec<FunctionShape>,
    function: Option<String>,
    identifiers: BTreeSet<IdentifierDeclaration>,
    pub(super) include_test_items: bool,
    pub(super) lint_metadata_failures: BTreeSet<String>,
    pub(super) type_context: TypeContext,
}

#[derive(Clone, Copy)]
pub(super) enum TypeContext {
    None,
    Structured,
    All,
}

impl SyntaxVisitor {
    pub(super) fn new(aliases: BTreeMap<String, AuthorityKind>, include_test_items: bool) -> Self {
        Self {
            aliases,
            authorities: BTreeSet::new(),
            closed_records: BTreeSet::new(),
            disallowed_lint_allowances: BTreeSet::new(),
            external_roots: BTreeSet::new(),
            functions: Vec::new(),
            function: None,
            identifiers: BTreeSet::new(),
            include_test_items,
            lint_metadata_failures: BTreeSet::new(),
            type_context: TypeContext::None,
        }
    }

    pub(super) fn report(self) -> RustSyntaxReport {
        RustSyntaxReport {
            authorities: self.authorities,
            closed_records: self.closed_records,
            disallowed_lint_allowances: self.disallowed_lint_allowances,
            external_roots: self.external_roots,
            functions: self.functions,
            identifiers: self.identifiers,
            lint_metadata_failures: self.lint_metadata_failures,
        }
    }

    pub(super) fn authority(&mut self, kind: AuthorityKind) {
        self.authorities.insert(AuthorityUse {
            function: self.function.clone(),
            kind,
        });
    }

    pub(super) fn identifier(&mut self, ident: &syn::Ident) {
        self.identifiers.insert(IdentifierDeclaration {
            name: ident.to_string(),
        });
    }

    pub(super) fn enter_function(
        &mut self,
        name: String,
        is_visible: bool,
        signature: &syn::Signature,
        body: Option<&syn::Block>,
    ) {
        self.functions.push(FunctionShape {
            name: name.clone(),
            returns_closed_result: closed_result(&signature.output),
        });
        let prior_function = self.function.replace(name);
        let prior_context = self.type_context;
        self.type_context = if is_visible || semantic_boundary_symbol(&signature.ident.to_string())
        {
            TypeContext::All
        } else {
            TypeContext::Structured
        };
        for input in &signature.inputs {
            self.visit_fn_arg(input);
        }
        self.visit_return_type(&signature.output);
        self.type_context = TypeContext::None;
        if let Some(block) = body {
            self.visit_block(block);
        }
        self.type_context = prior_context;
        self.function = prior_function;
    }

    pub(super) fn structured_parser(&mut self, node: &syn::Path) {
        self.authority(AuthorityKind::StructuredInput);
        for segment in &node.segments {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                continue;
            };
            for argument in &arguments.args {
                if let GenericArgument::Type(value) = argument {
                    for kind in authority_for_type(value, &self.aliases) {
                        self.authority(kind);
                    }
                }
            }
        }
    }
}
