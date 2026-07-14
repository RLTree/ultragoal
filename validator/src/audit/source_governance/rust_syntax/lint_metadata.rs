use std::collections::BTreeSet;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Lit, Meta, Token};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LintMetadataError {
    AllowShape,
    AllowValue,
    CfgAttrShape,
}

impl LintMetadataError {
    pub(super) fn stable_id(self) -> String {
        match self {
            Self::AllowShape => "allow_shape_invalid",
            Self::AllowValue => "allow_value_invalid",
            Self::CfgAttrShape => "cfg_attr_shape_invalid",
        }
        .to_string()
    }
}

pub(super) fn inspect(meta: &Meta) -> Result<BTreeSet<String>, LintMetadataError> {
    let mut lints = BTreeSet::new();
    inspect_attribute(meta, &mut lints)?;
    Ok(lints)
}

fn inspect_attribute(meta: &Meta, lints: &mut BTreeSet<String>) -> Result<(), LintMetadataError> {
    if meta.path().is_ident("allow") {
        return inspect_allowance(meta, lints);
    }
    if meta.path().is_ident("cfg_attr") {
        return inspect_cfg_attr(meta, lints);
    }
    Ok(())
}

fn inspect_allowance(meta: &Meta, lints: &mut BTreeSet<String>) -> Result<(), LintMetadataError> {
    let Meta::List(list) = meta else {
        return Err(LintMetadataError::AllowShape);
    };
    let entries = parse_meta_list(list).map_err(|_| LintMetadataError::AllowShape)?;
    if entries.is_empty() {
        return Err(LintMetadataError::AllowShape);
    }
    let mut lint_count = 0;
    let mut reason_seen = false;
    for entry in entries {
        match entry {
            Meta::Path(path) => {
                lint_count += 1;
                let lint = path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if prohibited(&lint) {
                    lints.insert(lint);
                }
            }
            Meta::NameValue(value) if valid_reason(&value) && !reason_seen => {
                reason_seen = true;
            }
            Meta::List(_) | Meta::NameValue(_) => return Err(LintMetadataError::AllowValue),
        }
    }
    if lint_count == 0 {
        Err(LintMetadataError::AllowValue)
    } else {
        Ok(())
    }
}

fn inspect_cfg_attr(meta: &Meta, lints: &mut BTreeSet<String>) -> Result<(), LintMetadataError> {
    let Meta::List(list) = meta else {
        return Err(LintMetadataError::CfgAttrShape);
    };
    let entries = parse_meta_list(list).map_err(|_| LintMetadataError::CfgAttrShape)?;
    if entries.len() < 2 {
        return Err(LintMetadataError::CfgAttrShape);
    }
    inspect_nested_metadata(&entries[0], lints)?;
    for nested in entries.iter().skip(1) {
        inspect_attribute(nested, lints)?;
    }
    Ok(())
}

fn inspect_nested_metadata(
    meta: &Meta,
    lints: &mut BTreeSet<String>,
) -> Result<(), LintMetadataError> {
    if meta.path().is_ident("allow") || meta.path().is_ident("cfg_attr") {
        return inspect_attribute(meta, lints);
    }
    let Meta::List(list) = meta else {
        return Ok(());
    };
    let entries = parse_meta_list(list).map_err(|_| LintMetadataError::CfgAttrShape)?;
    for nested in entries {
        inspect_nested_metadata(&nested, lints)?;
    }
    Ok(())
}

fn parse_meta_list(list: &syn::MetaList) -> Result<Punctuated<Meta, Token![,]>, syn::Error> {
    Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())
}

fn valid_reason(value: &syn::MetaNameValue) -> bool {
    value.path.is_ident("reason")
        && matches!(&value.value, Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) if !text.value().trim().is_empty())
}

fn prohibited(lint: &str) -> bool {
    matches!(
        lint,
        "dead_code"
            | "unreachable_code"
            | "unused"
            | "unused_imports"
            | "warnings"
            | "clippy::all"
            | "clippy::cargo"
            | "clippy::complexity"
            | "clippy::correctness"
            | "clippy::nursery"
            | "clippy::pedantic"
            | "clippy::perf"
            | "clippy::restriction"
            | "clippy::style"
            | "clippy::suspicious"
            | "clippy::too_many_arguments"
            | "clippy::type_complexity"
    )
}
