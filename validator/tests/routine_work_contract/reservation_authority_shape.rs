#[path = "reservation_authority_shape/declarations.rs"]
mod declarations;
#[path = "reservation_authority_shape/exported_methods.rs"]
mod exported_methods;

use syn::visit::Visit;

const AUTHORITY_SOURCE: &str =
    include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs");
const CUSTODY_SOURCE: &str = include_str!(
    "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/custody.rs"
);
const STAGED_SOURCE: &str = include_str!(
    "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/staged_custody.rs"
);
const FLAG_SOURCE: &str = include_str!(
    "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/transition_flag.rs"
);

pub(super) fn assert_current() {
    let parent =
        include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/mod.rs");
    assert!(parent.contains("mod authority;"));
    assert!(parent.contains("pub(super) use authority::{AttemptReservation, reserve_grant};"));
    assert!(!parent.contains("mod staged_custody;"));
    assert_eq!(
        validate(AUTHORITY_SOURCE, CUSTODY_SOURCE, STAGED_SOURCE, FLAG_SOURCE,),
        Ok(())
    );
}

pub(super) fn validate(
    authority: &str,
    custody: &str,
    staged: &str,
    flag: &str,
) -> Result<(), &'static str> {
    let authority = syn::parse_file(authority).map_err(|_| "authority-parse")?;
    let custody = syn::parse_file(custody).map_err(|_| "custody-parse")?;
    let staged = syn::parse_file(staged).map_err(|_| "staged-parse")?;
    let flag = syn::parse_file(flag).map_err(|_| "flag-parse")?;
    declarations::validate(&authority, &custody, &staged, &flag)?;
    exported_methods::validate(&authority, &custody, &staged, &flag)?;
    validate_bodies([&authority, &custody, &staged, &flag])
}

fn validate_bodies(files: [&syn::File; 4]) -> Result<(), &'static str> {
    let mut visitor = BodyBoundary(true);
    for file in files {
        for item in &file.items {
            match item {
                syn::Item::Fn(function) => visitor.visit_block(&function.block),
                syn::Item::Impl(implementation) => implementation.items.iter().for_each(|member| {
                    if let syn::ImplItem::Fn(method) = member {
                        visitor.visit_block(&method.block);
                    }
                }),
                _ => {}
            }
        }
    }
    visitor.0.then_some(()).ok_or("authority-hidden-code")
}

struct BodyBoundary(bool);

impl<'ast> Visit<'ast> for BodyBoundary {
    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if matches!(statement, syn::Stmt::Item(_) | syn::Stmt::Macro(_)) {
            self.0 = false;
        } else {
            syn::visit::visit_stmt(self, statement);
        }
    }

    fn visit_expr_unsafe(&mut self, _expression: &'ast syn::ExprUnsafe) {
        self.0 = false;
    }

    fn visit_macro(&mut self, _macro: &'ast syn::Macro) {
        self.0 = false;
    }
}
