use super::model::CompilePossibility;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Lit, Meta, Token};

pub(super) fn possibility(attributes: &[Attribute]) -> Result<CompilePossibility, &'static str> {
    let mut out = CompilePossibility::BOTH;
    for attribute in attributes {
        if attribute.path().is_ident("cfg") {
            let predicate = single_meta(attribute)?;
            out = out.and(CompilePossibility {
                production: truth(&predicate, false)?.can_true,
                test: truth(&predicate, true)?.can_true,
            });
        } else if attribute.path().is_ident("cfg_attr")
            && conditional_authority_attribute(attribute)?
        {
            return Err("conditional_cfg_or_path_attribute_unsupported");
        }
    }
    Ok(out)
}

pub(super) fn path_attribute(attributes: &[Attribute]) -> Result<Option<String>, &'static str> {
    let mut found = None;
    for attribute in attributes {
        if attribute.path().is_ident("path") {
            if found.is_some() {
                return Err("duplicate_path_attribute");
            }
            let Meta::NameValue(value) = &attribute.meta else {
                return Err("path_attribute_shape_invalid");
            };
            let Expr::Lit(expression) = &value.value else {
                return Err("path_attribute_value_invalid");
            };
            let Lit::Str(path) = &expression.lit else {
                return Err("path_attribute_value_invalid");
            };
            found = Some(path.value());
        }
    }
    Ok(found)
}

fn single_meta(attribute: &Attribute) -> Result<Meta, &'static str> {
    let list = attribute
        .meta
        .require_list()
        .map_err(|_| "cfg_attribute_shape_invalid")?;
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let values = parser
        .parse2(list.tokens.clone())
        .map_err(|_| "cfg_predicate_invalid")?;
    if values.len() != 1 {
        return Err("cfg_predicate_arity_invalid");
    }
    values
        .into_iter()
        .next()
        .ok_or("cfg_predicate_arity_invalid")
}

fn conditional_authority_attribute(attribute: &Attribute) -> Result<bool, &'static str> {
    let list = attribute
        .meta
        .require_list()
        .map_err(|_| "cfg_attr_shape_invalid")?;
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let values = parser
        .parse2(list.tokens.clone())
        .map_err(|_| "cfg_attr_predicate_invalid")?;
    if values.len() < 2 {
        return Err("cfg_attr_arity_invalid");
    }
    Ok(values.iter().skip(1).any(authority_meta))
}

fn authority_meta(meta: &Meta) -> bool {
    if meta.path().is_ident("cfg") || meta.path().is_ident("path") {
        return true;
    }
    let Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    parser
        .parse2(list.tokens.clone())
        .ok()
        .is_none_or(|values| values.iter().skip(1).any(authority_meta))
}

#[derive(Clone, Copy)]
struct TruthPossibility {
    can_true: bool,
    can_false: bool,
}

fn truth(meta: &Meta, test: bool) -> Result<TruthPossibility, &'static str> {
    match meta {
        Meta::Path(path) if path.is_ident("test") => Ok(fixed(test)),
        Meta::Path(_) | Meta::NameValue(_) => Ok(unknown()),
        Meta::List(list) if list.path.is_ident("all") => {
            let values = nested(list)?;
            let states = values
                .iter()
                .map(|value| truth(value, test))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TruthPossibility {
                can_true: states.iter().all(|state| state.can_true),
                can_false: states.iter().any(|state| state.can_false),
            })
        }
        Meta::List(list) if list.path.is_ident("any") => {
            let values = nested(list)?;
            let states = values
                .iter()
                .map(|value| truth(value, test))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TruthPossibility {
                can_true: states.iter().any(|state| state.can_true),
                can_false: states.iter().all(|state| state.can_false),
            })
        }
        Meta::List(list) if list.path.is_ident("not") => {
            let values = nested(list)?;
            if values.len() != 1 {
                return Err("cfg_not_arity_invalid");
            }
            let state = truth(&values[0], test)?;
            Ok(TruthPossibility {
                can_true: state.can_false,
                can_false: state.can_true,
            })
        }
        Meta::List(_) => Ok(unknown()),
    }
}

fn nested(list: &syn::MetaList) -> Result<Vec<Meta>, &'static str> {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    parser
        .parse2(list.tokens.clone())
        .map(|values| values.into_iter().collect())
        .map_err(|_| "cfg_nested_predicate_invalid")
}

fn fixed(value: bool) -> TruthPossibility {
    TruthPossibility {
        can_true: value,
        can_false: !value,
    }
}

fn unknown() -> TruthPossibility {
    TruthPossibility {
        can_true: true,
        can_false: true,
    }
}
