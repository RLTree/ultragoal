mod adapter;
mod classification;
mod lint_metadata;
mod model;
mod use_aliases;
mod visitor;
mod visitor_state;

pub(crate) use adapter::{RustSyntaxReport, RustSyntaxRequest, analyze};
pub(crate) use model::{AuthorityKind, FunctionShape};
