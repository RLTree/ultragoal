use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Lit, Meta, Token};

pub(super) fn path(attributes: &[Attribute]) -> Result<Option<String>, &'static str> {
    let mut found = None;
    for attribute in attributes {
        if attribute.path().is_ident("cfg_attr") && conditional_authority(&attribute.meta)? {
            return Err("conditional_cfg_or_path_attribute_unsupported");
        }
        if !attribute.path().is_ident("path") {
            continue;
        }
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
        if path.value().is_empty() {
            return Err("path_attribute_empty");
        }
        found = Some(path.value());
    }
    Ok(found)
}

fn conditional_authority(meta: &Meta) -> Result<bool, &'static str> {
    if meta.path().is_ident("cfg") || meta.path().is_ident("path") {
        return Ok(true);
    }
    if !meta.path().is_ident("cfg_attr") {
        return Ok(false);
    }
    let Meta::List(list) = meta else {
        return Err("cfg_attr_shape_invalid");
    };
    let values = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .map_err(|_| "cfg_attr_predicate_invalid")?;
    if values.len() < 2 {
        return Err("cfg_attr_arity_invalid");
    }
    for value in values.iter().skip(1) {
        if conditional_authority(value)? {
            return Ok(true);
        }
    }
    Ok(false)
}
